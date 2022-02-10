use core::arch::asm;
use std::thread;

extern "C" {
    fn ioperm(from: u64, num: u64, turn_on: i32) -> i32;
}

unsafe fn read_port(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

unsafe fn write_port(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

const EC_SC: u16 = 0x66;
const EC_DATA: u16 = 0x62;
const IBF: u8 = 1;
const EC_REG_FAN_DUTY: u8 = 0xCE;
const EC_SC_READ_CMD: u8 = 0x80;
const EC_REG_CPU_TEMP: u8 = 0x07;

fn ec_io_do(cmd: u8, port: u8, value: u8) -> bool {
    ec_io_wait();
    unsafe { write_port(EC_SC, cmd) }

    ec_io_wait();
    unsafe { write_port(EC_DATA, port) }

    ec_io_wait();
    unsafe { write_port(EC_DATA, value) }

    ec_io_wait()
}

pub fn ec_write_fan_duty(percent: f32) -> bool {
    let rdb = calculate_raw_duty(percent);
    ec_io_do(0x99, 0x01, rdb)
}

pub fn ec_query_cpu_temp() -> u8 {
    ec_io_read(EC_REG_CPU_TEMP)
}

pub fn calculate_fan_duty(raw_duty: u8) -> f32 {
    (raw_duty as f32 / 255.0) * 100.0
}

pub fn calculate_raw_duty(percent: f32) -> u8 {
    ((percent / 100.0) * 255.0) as u8
}

pub fn ec_query_fan_duty() -> f32 {
    let raw_duty = ec_io_read(EC_REG_FAN_DUTY);
    calculate_fan_duty(raw_duty)
}

fn ec_io_read(port: u8) -> u8 {
    ec_io_wait();
    unsafe { write_port(EC_SC, EC_SC_READ_CMD) }

    ec_io_wait();
    unsafe { write_port(EC_DATA, port) }
    ec_io_wait();

    unsafe { read_port(EC_DATA) }
}

fn ec_io_wait() -> bool {
    let mut data = unsafe { read_port(EC_SC) };

    let mut i = 0;
    while ((data >> IBF) & 0x1) != 0 {
        thread::sleep(std::time::Duration::from_micros(1000));
        data = unsafe { read_port(EC_SC) };
        if i == 100 {
            eprintln!("err on waiting for the ec io");
            return false;
        }
        i += 1;
    }
    true
}

pub fn ec_init() -> bool {
    if unsafe { ioperm(EC_DATA as u64, 1, 1) != 0 } {
        return false;
    }
    if unsafe { ioperm(EC_SC as u64, 1, 1) != 0 } {
        return false;
    }
    true
}
