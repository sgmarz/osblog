// vfs.rs
// Virtual File System
// Stephen Marz
// 4 June 2020

use crate::fs::{MinixFileSystem, Inode, S_IFDIR, BLOCK_SIZE, DirEntry, FsError};
use crate::buffer::Buffer;
use crate::lock::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::mem::size_of;

#[derive(Clone, Copy)]
pub enum EntryType {
    File(Inode),
    Block,
    Char,
    Fifo,
    Socket,
    Gpu,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub dev: usize,
    pub node: EntryType,
}

static mut LOCK: Mutex = Mutex::new();
static mut CACHE: Option<BTreeMap<String, Entry>> = None;

/// Init is where we would cache the superblock and inode to avoid having to read
/// it over and over again, like we do for read right now.
fn cache_at(btm: &mut BTreeMap<String, Entry>, cwd: &String, inode_num: u32, bdev: usize) {
    let ino_opt = MinixFileSystem::get_inode(bdev, inode_num);
    if ino_opt.is_none() {
        println!("Error getting inode {}: '{}'", inode_num, cwd);
        return;
    }
    let ino = ino_opt.unwrap();
    let mut buf = Buffer::new(((ino.size + BLOCK_SIZE - 1) & !BLOCK_SIZE) as usize);
    let dirents = buf.get() as *const DirEntry;
    let sz = MinixFileSystem::read(bdev, &ino, buf.get_mut(), BLOCK_SIZE, 0);
    let num_dirents = sz as usize / size_of::<DirEntry>();
    // We start at 2 because the first two entries are . and ..
    for i in 2..num_dirents {
        unsafe {
            let ref d = *dirents.add(i);
            let d_ino = MinixFileSystem::get_inode(bdev, d.inode).unwrap();
            let mut new_cwd = String::with_capacity(120);
            for i in cwd.bytes() {
                new_cwd.push(i as char);
            }
            // Add a directory separator between this inode and the next.
            // If we're the root (inode 1), we don't want to double up the
            // frontslash, so only do it for non-roots.
            if inode_num != 1 {
                new_cwd.push('/');
            }
            for i in 0..60 {
                if d.name[i] == 0 {
                    break;
                }
                new_cwd.push(d.name[i] as char);
            }
            // new_cwd.shrink_to_fit();
            if d_ino.mode & S_IFDIR != 0 {
                // This is a directory, cache these. This is a recursive call,
                // which I don't really like.
                cache_at(btm, &new_cwd, d.inode, bdev);
            }
            let ent = Entry {
                dev: bdev,
                node: EntryType::File(d_ino),
            };
            btm.insert(new_cwd, ent);
        }
    }
}

// Run this ONLY in a process!
pub fn init(bdev: usize) {
    unsafe {
        LOCK.spin_lock();
    }
    if unsafe { CACHE.is_none() } {
        unsafe {
            CACHE.replace(BTreeMap::new());
        }
    }
    if let Some(mut btm) = unsafe { CACHE.take() } {
        let cwd = String::from("/");

        // Let's look at the root (inode #1)
        cache_at(&mut btm, &cwd, 1, bdev);
        unsafe {
            CACHE = Some(btm);
        }
    }
    else {
        panic!("KERNEL: Initialized an already initialized filesystem {}", bdev);
    }
    unsafe {
        LOCK.unlock();
    }
}

pub fn open(path: &str) -> Result<Entry, FsError> {
    let ret;
    // We use an atomic lock here. Try lock will attempt the lock
    // but will not wait if it can't get it. It returns true
    // if we acquire the lock or false otherwise.
    if unsafe { LOCK.try_lock() } == false {
        return Err(FsError::Busy);
    }
    else if let Some(cache) = unsafe { CACHE.take() } {
        // If we get here, we've locked the mutex and were able to take the
        // global cache.
        if let Some(entry) = cache.get(path) {
            // If we get here, we have the lock, cache, and we found
            // the requested key (directory path)
            ret = Ok(*entry);
        }
        else {
            // If we get here, we didn't find the path in the cache.
            ret = Err(FsError::FileNotFound);
        }
        unsafe {
            CACHE.replace(cache);
        }
    }
    else {
        // If we get here, we got the lock, but couldn't take
        // the cache.
        ret = Err(FsError::Unavailable);
    }
    unsafe {
        LOCK.unlock();
    }
    ret
}

pub fn init_proc(dev: usize) {
	init(dev);
}

pub fn add_gpu(dev: usize) {
    if unsafe { LOCK.try_lock() } == false {
        println!("add_gpu: Could not acquire lock.");
        return;
    }
    if let Some(mut cache) = unsafe { CACHE.take() } {
        let entry = Entry {
            dev,
            node: EntryType::Gpu,
        };
        cache.insert(String::from("/dev/gpu"), entry);
        unsafe {
            CACHE.replace(cache);
        }
    }
    unsafe {
        LOCK.unlock();
    }
}
