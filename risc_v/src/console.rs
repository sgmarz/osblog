// console.rs
// Console utilities for buffering
// Stephen Marz
// 4 June 2020

use alloc::collections::VecDeque;
use crate::lock::Mutex;
use crate::process::{get_by_pid, set_running};

pub static mut IN_BUFFER: Option<VecDeque<u8>> = None;
pub static mut OUT_BUFFER: Option<VecDeque<u8>> = None;

pub static mut IN_LOCK: Mutex = Mutex::new();
pub static mut OUT_LOCK: Mutex = Mutex::new();

pub const DEFAULT_OUT_BUFFER_SIZE: usize = 10_000;
pub const DEFAULT_IN_BUFFER_SIZE: usize = 1_000;

pub static mut CONSOLE_QUEUE: Option<VecDeque<u16>> = None;

pub fn init() {
    unsafe {
        IN_BUFFER.replace(VecDeque::with_capacity(DEFAULT_IN_BUFFER_SIZE));
        OUT_BUFFER.replace(VecDeque::with_capacity(DEFAULT_OUT_BUFFER_SIZE));
    }
}

/// Push a u8 (character) onto the output buffer
/// If the buffer is full, silently drop.
pub fn push_stdout(c: u8) {
    unsafe {
        OUT_LOCK.spin_lock();
        if let Some(mut buf) = OUT_BUFFER.take() {
            if buf.len() < DEFAULT_OUT_BUFFER_SIZE {
                buf.push_back(c);
            }
            OUT_BUFFER.replace(buf);
        }
        OUT_LOCK.unlock();
    }
}

pub fn pop_stdout() -> u8 {
    let mut ret = None;
    unsafe {
        OUT_LOCK.spin_lock();
        if let Some(mut buf) = OUT_BUFFER.take() {
            ret = buf.pop_front();
            OUT_BUFFER.replace(buf);
        }
        OUT_LOCK.unlock();
    }
    ret.unwrap_or(0)
}

pub fn push_stdin(c: u8) {
    unsafe {
        IN_LOCK.spin_lock();
        if let Some(mut buf) = IN_BUFFER.take() {
            if buf.len() < DEFAULT_IN_BUFFER_SIZE {
                buf.push_back(c);
                if c == 10 || c == 11 {
                    if let Some(mut q) = CONSOLE_QUEUE.take() {
                        for i in q.drain(..) {
                            set_running(i);
                            // We also need to put stuff in here.
                        }
                        CONSOLE_QUEUE.replace(q);
                    }
                }
            }
            IN_BUFFER.replace(buf);
        }
        IN_LOCK.unlock();
    }
}

pub fn pop_stdin() -> u8 {
    let mut ret = None;
    unsafe {
        IN_LOCK.spin_lock();
        if let Some(mut buf) = IN_BUFFER.take() {
            ret = buf.pop_front();
            IN_BUFFER.replace(buf);
        }
        IN_LOCK.unlock();
    }
    ret.unwrap_or(0)
}

pub fn push_queue(pid: u16) {
    unsafe {
        if let Some(mut q) = CONSOLE_QUEUE.take() {
            q.push_back(pid);
            CONSOLE_QUEUE.replace(q);
        }
    }
}
