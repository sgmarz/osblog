// console.rs
// Console utilities for buffering
// Stephen Marz
// 4 June 2020

use alloc::collections::VecDeque;
use crate::uart::Uart;
use crate::lock::Mutex;
use crate::syscall::syscall_yield;

pub static mut READ_BUFFER: Option<VecDeque<u8>> = None;
pub static mut WRITE_BUFFER: Option<VecDeque<u8>> = None;

pub static mut READ_HANDLER: fn() -> u8 = uart_read;
pub static mut WRITE_HANDLER: fn(u8) = uart_write;

pub static mut READ_LOCK: Mutex = Mutex::new();
pub static mut WRITE_LOCK: Mutex = Mutex::new();


fn uart_read() -> u8 {
    let mut u = Uart::new(0x1000_0000);
    if let Some(c) = u.get() {
        c
    }
    else {
        0
    }
}

fn uart_write(c: u8) {
    let mut u = Uart::new(0x1000_0000);
    u.put(c);
}

pub fn init() {
    unsafe {
        WRITE_BUFFER.replace(VecDeque::new());
        READ_BUFFER.replace(VecDeque::new());
    }
}

pub fn console_read_proc() {
    loop {
        unsafe {
            READ_LOCK.sleep_lock();
            if let Some(mut cb) = READ_BUFFER.take() {
                let ur = READ_HANDLER();
                if ur != 0 {
                    cb.push_back(ur);
                }
                READ_BUFFER.replace(cb);
            }
            READ_LOCK.unlock();
        }
        syscall_yield();
    }
}

pub fn console_write_proc() {
    loop {
        unsafe {
            WRITE_LOCK.sleep_lock();
            if let Some(mut cb) = WRITE_BUFFER.take() {
                while let Some(c) = cb.pop_front() {
                    WRITE_HANDLER(c);
                }
                WRITE_BUFFER.replace(cb);
            }
            WRITE_LOCK.unlock();
        }
        syscall_yield();
    }
}
