// virtio.rs
// VirtIO routines for the VirtIO protocol
// Stephen Marz
// 10 March 2020

// Flags
// Descriptor flags have VIRTIO_DESC_F as a prefix
// Available flags have VIRTIO_AVAIL_F

pub const VIRTIO_DESC_F_NEXT: u16 = 1;
pub const VIRTIO_DESC_F_WRITE: u16 = 2;
pub const VIRTIO_DESC_F_INDIRECT: u16 = 4;

pub const VIRTIO_AVAIL_F_NO_INTERRUPT: u16 = 1;

pub const VIRTIO_USED_F_NO_NOTIFY: u16 = 1;

// VirtIO structures
#[repr(C)]
pub struct Descriptor {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
pub struct Available {
    flags: u16,
    idx: u16,
    ring: [u16; 1024],
    event: u16,
}

#[repr(C)]
pub struct UsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
pub struct Used {
    flags: u16,
    idx: u16,
    ring: [UsedElem; 1024],
    event: u16,
}

// We probably shouldn't put these here, but it'll help
// with probing the bus, etc. These are architecture specific
// which is why I say that.
pub const MMIO_VIRTIO_START: usize  = 0x1000_1000;
pub const MMIO_VIRTIO_END: usize    = 0x1000_8000;
pub const MMIO_VIRTIO_STRIDE: usize = 0x1000;

/// Probe the VirtIO bus for devices that might be
/// out there.
pub fn probe() {
    // Rust's for loop uses an Iterator object, which now has a step_by modifier
    // to change how much it steps. Also recall that ..= means up to AND including
    // MMIO_VIRTIO_END.
    for addr in (MMIO_VIRTIO_START..=MMIO_VIRTIO_END).step_by(MMIO_VIRTIO_STRIDE) {
        print!("Virtio probing 0x{:08x}...", addr);
        unsafe {
            let ptr = addr as *mut u32;
            // 0x74_72_69_76 is "virt" in little endian, so in reality
            // it is triv. All VirtIO devices have this attached to the
            // MagicValue register (offset 0x000)
            if 0x74_72_69_76 != ptr.read_volatile() {
                println!("not virtio.");
            }
            // If we are a virtio device, we now need to see if anything
            // is actually attached to it. The DeviceID register will
            // contain what type of device this is. If this value is 0,
            // then it is not connected.
            else if 0 == ptr.add(2).read_volatile() {
                println!("not connected.");
            }
            else {
                match ptr.add(2).read_volatile() {
                    // DeviceID 1 is a network device
                    1 => {
                        print!("network device...");
                        if false == setup_network_device(ptr) {
                            println!("setup failed.");
                        }
                        else {
                            println!("setup succeeded!");
                        }
                    },
                    // DeviceID 2 is a block device
                    2 => {
                        print!("block device...");
                        if false == setup_block_device(ptr) {
                            println!("setup failed.");
                        }
                        else {
                            println!("setup succeeded!");
                        }
                    },
                    // DeviceID 4 is a random number generator device
                    4 => {
                        print!("entropy device...");
                        if false == setup_entropy_device(ptr) {
                            println!("setup failed.");
                        }
                        else {
                            println!("setup succeeded!");
                        }
                    },
                    // DeviceID 16 is a GPU device
                    16 => {
                        print!("GPU device...");
                        if false == setup_gpu_device(ptr) {
                            println!("setup failed.");
                        }
                        else {
                            println!("setup succeeded!");
                        }
                    },
                    // DeviceID 18 is an input device
                    18 => {
                        print!("input device...");
                        if false == setup_input_device(ptr) {
                            println!("setup failed.");
                        }
                        else {
                            println!("setup succeeded!");
                        }
                    },
                    _ => {
                        println!("unknown device type.")
                    }
                }
            }
        }
    }
}

pub fn setup_block_device(ptr: *mut u32) -> bool {
    false
}

pub fn setup_entropy_device(ptr: *mut u32) -> bool {
    false
}

pub fn setup_network_device(ptr: *mut u32) -> bool {
    false
}

pub fn setup_gpu_device(ptr: *mut u32) -> bool {
    false
}

pub fn setup_input_device(ptr: *mut u32) -> bool {
    false
}
