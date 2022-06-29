use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::prelude::FileExt;
use std::process::Command;
use std::thread;
use std::time::Duration;

mod error;
use error::Error;

mod ec_io;
use ec_io::*;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

static mut QUIT: bool = false;

const TIMEOUT: u64 = 2;
const EC_REG_SIZE: usize = 0x100;
const EC_REG_FAN_DUTY: usize = 0xCE;
const EC_REG_CPU_TEMP: usize = 0x07;
const EC_REG_GPU_TEMP: usize = 0xCD;

fn calc_next_duty(temp: f32) -> f32 {
    if temp <= 40.0 {
        32.0
    } else if temp <= 60.0 {
        0.71 * temp + 4.0
    } else if temp <= 80.0 {
        2.5 * temp - 100.0
    } else {
        100.0
    }
}

fn calc_next_duty_quiet(temp: f32) -> f32 {
    if temp <= 40.0 {
        28.0
    } else if temp <= 50.0 {
        0.58 * temp + 5.0
    } else if temp <= 65.0 {
        1.1 * temp - 20.0
    } else if temp <= 80.0 {
        2.2 * temp - 88.0
    } else {
        100.0
    }
}

struct EC<'a> {
    pub fan_duty: u8,
    pub fan_next_duty: u32,
    pub cpu_temp: u8,
    pub gpu_temp: u8,
    pub duty_calc_func: &'a dyn Fn(f32) -> f32,
    handle: File,
    i: u8,
    i2: u8,
}
impl<'a> EC<'a> {
    const MAX_STEP: u32 = 8;
    const LOWER_END: u32 = 10;
    const HIGHER_END: u32 = 5;

    pub fn new(duty_calc_func: &'a dyn Fn(f32) -> f32) -> io::Result<Self> {
        let handle = OpenOptions::new()
            .read(true)
            .write(false)
            .open("/sys/kernel/debug/ec/ec0/io")?;

        Ok(Self {
            fan_duty: 0,
            fan_next_duty: 0,
            cpu_temp: 0,
            gpu_temp: 0,
            duty_calc_func,
            handle,
            i: 0,
            i2: 0,
        })
    }

    pub fn read_ec(&mut self) -> io::Result<()> {
        let mut buf = [0_u8; EC_REG_SIZE];
        self.handle.read_exact_at(&mut buf, 0)?;
        self.fan_duty = calculate_fan_duty(buf[EC_REG_FAN_DUTY]) as u8;
        self.cpu_temp = buf[EC_REG_CPU_TEMP];
        // self.gpu_temp = buf[EC_REG_GPU_TEMP];
        Ok(())
    }

    pub fn switch_to_next_duty(&mut self) -> Option<bool> {
        let fan = (self.duty_calc_func)(self.cpu_temp as f32) as u32;
        let current_fd = self.fan_duty as u32;

        if self.i >= 4
            || !(fan >= current_fd - Self::LOWER_END && fan <= current_fd + Self::HIGHER_END)
        {
            self.i = 0;
            let next_duty: u32;

            if fan.abs_diff(current_fd) <= 3 {
                return None;
            } else if fan > current_fd {
                next_duty = fan;
            } else if self.i2 >= 4 {
                self.i2 = 0;
                let step = std::cmp::min(current_fd - fan, Self::MAX_STEP);
                next_duty = current_fd - step;
            } else {
                self.i2 += 1;
                return None;
            }
            self.fan_next_duty = next_duty;
            return Some(ec_write_fan_duty(next_duty));
        }
        self.i += 1;
        None
    }

    pub fn load_module() -> io::Result<()> {
        Command::new("/sbin/modprobe")
            .arg("ec_sys")
            .spawn()?
            .wait()?;
        Ok(())
    }
}

enum Mode {
    Default,
    Quiet,
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mode::Default => write!(f, "default mode"),
            Mode::Quiet => write!(f, "quiet mode"),
        }
    }
}

fn main() -> Result<(), Error> {
    const DEFAULT_MODE_ARG: &str = "--default";
    const QUIET_MODE_ARG: &str = "--quiet";

    if !ec_init() {
        return Err(Error::NoSudo);
    }

    let mut args = std::env::args();
    let executable_name = args.next().ok_or(Error::NoExecName)?;

    let mode = match args.next() {
        Some(arg) => match arg.as_str() {
            QUIET_MODE_ARG => Mode::Quiet,
            DEFAULT_MODE_ARG => Mode::Default,
            _ => return Err(Error::WrongArgs(executable_name)),
        },
        None => Mode::Default,
    };

    println!("Running in {mode}");

    set_handlers();

    println!(
        "initial: fan={}%, CPU={}°C",
        ec_query_fan_duty(),
        ec_query_cpu_temp()
    );

    EC::load_module()?;
    let mut ec = match mode {
        Mode::Default => EC::new(&calc_next_duty),
        Mode::Quiet => EC::new(&calc_next_duty_quiet),
    }?;

    ec.read_ec().map_err(Error::ErrRead)?;
    println!("initial ec: fan={}%, CPU={}°C", ec.fan_duty, ec.cpu_temp);

    while !unsafe { QUIT } {
        ec.read_ec().map_err(Error::ErrRead)?;

        if let Some(s) = ec.switch_to_next_duty() {
            if s {
                println!(
                    "current: fan={}%, CPU={}°C, next: fan={}%",
                    ec.fan_duty, ec.cpu_temp, ec.fan_next_duty
                );
            } else {
                return Err(Error::ErrWrite);
            }
        }
        thread::sleep(Duration::from_secs(TIMEOUT));
    }
    Ok(())
}

fn sighandler(_: i32) {
    println!("Signal received. Waiting for EC, then closing..");
    unsafe { QUIT = true }
}

fn set_handlers() {
    let p_sighandler = sighandler as usize;
    unsafe {
        // handle-able signals
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
