use std::fs::File;
use std::io::{self, Read};
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

mod ec_io;
pub use ec_io::*;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

static mut QUIT: bool = false;

const EC_REG_SIZE: usize = 0x100;
const EC_REG_FAN_DUTY: usize = 0xCE;
const EC_REG_CPU_TEMP: usize = 0x07;

const SLEEP: u64 = 2;

fn calc_next_duty(temp: f32) -> f32 {
    if temp <= 40.0 {
        38.0
    } else if temp <= 60.0 {
        0.75 * temp + 5.0
    } else if temp <= 80.0 {
        2.5 * temp - 100.0
    } else {
        100.0
    }
}

#[derive(Default)]
pub struct EC {
    pub fan_duty: u8,
    pub cpu_temp: u8,
    c: u8,
}
impl EC {
    const MAX_STEP: u16 = 8;
    const LOWER_END: u16 = 10;
    const HIHGER_END: u16 = 5;

    pub fn read_from_kernel(&mut self) -> io::Result<()> {
        let mut f = File::open("/sys/kernel/debug/ec/ec0/io")?;
        let mut buf = [0_u8; EC_REG_SIZE];
        f.read_exact(&mut buf)?;
        self.fan_duty = calculate_fan_duty(buf[EC_REG_FAN_DUTY]) as u8;
        self.cpu_temp = buf[EC_REG_CPU_TEMP];
        Ok(())
    }

    pub fn switch_to_next_duty(&mut self) -> bool {
        let fan = calc_next_duty(self.cpu_temp as f32) as u16;
        let current_fd = self.fan_duty as u16;

        if !(fan >= current_fd - Self::LOWER_END && fan <= current_fd + Self::HIHGER_END) {
            let next_duty = if fan >= current_fd {
                fan
            } else if self.c >= 5 {
                self.c = 0;
                let s = std::cmp::min(current_fd - fan, Self::MAX_STEP);
                current_fd - s
            } else {
                self.c += 1;
                current_fd
            };
            return ec_write_fan_duty(next_duty as f32);
        }
        true
    }

    pub fn load_module() -> io::Result<()> {
        Command::new("/sbin/modprobe")
            .arg("ec_sys")
            .spawn()?
            .wait()?;
        Ok(())
    }
}

fn main() {
    set_handlers();

    if !ec_init() {
        eprintln!("run with sudo");
        process::exit(1);
    }

    println!(
        "initial: fan={}%, CPU={}°C",
        ec_query_fan_duty(),
        ec_query_cpu_temp()
    );

    EC::load_module().unwrap();
    let mut ec = EC::default();
    ec.read_from_kernel().unwrap();
    println!("initial ec: fan={}%, CPU={}°C", ec.fan_duty, ec.cpu_temp);

    let s = Duration::from_secs(SLEEP);
    while !unsafe { QUIT } {
        if let Err(e) = ec.read_from_kernel() {
            eprintln!("err on reading: '{e}'");
            break;
        }
        if !ec.switch_to_next_duty() {
            eprintln!("err on writing to the ec fan duty");
            break;
        }
        println!("next: fan={}%, CPU={}°C", ec.fan_duty, ec.cpu_temp);

        thread::sleep(s);
    }
}

fn sighandler(_: i32) {
    println!("Signal received. Waiting for EC, then closing..");
    unsafe { QUIT = true }
}

fn set_handlers() {
    let p_sighandler = sighandler as usize;
    unsafe {
        signal(1, p_sighandler);
        signal(2, p_sighandler);
        signal(3, p_sighandler);
        signal(10, p_sighandler);
        signal(12, p_sighandler);
        signal(13, p_sighandler);
        signal(14, p_sighandler);
        signal(15, p_sighandler);
    }
}
