// fs.rs
// Filesystem for SOS
// Stephen Marz
// 16 March 2020

use alloc::string::String;

pub trait FileSystem {
    fn open(path: &String) -> Option<Descriptor>;
    fn read(desc: &Descriptor, buffer: *mut u8, offset: u32, size: u32) -> u32;
}

/// A file descriptor
pub struct Descriptor {

}
