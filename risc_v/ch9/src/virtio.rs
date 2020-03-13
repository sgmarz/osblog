// virtio.rs
// VirtIO routines for the VirtIO protocol
// Stephen Marz
// 10 March 2020

use crate::block::setup_block_device;
use crate::block;

// Flags
// Descriptor flags have VIRTIO_DESC_F as a prefix
// Available flags have VIRTIO_AVAIL_F

pub const VIRTIO_DESC_F_NEXT: u16 = 1;
pub const VIRTIO_DESC_F_WRITE: u16 = 2;
pub const VIRTIO_DESC_F_INDIRECT: u16 = 4;

pub const VIRTIO_AVAIL_F_NO_INTERRUPT: u16 = 1;

pub const VIRTIO_USED_F_NO_NOTIFY: u16 = 1;

pub const VIRTIO_RING_SIZE: usize = 1024;

// VirtIO structures
#[repr(C)]
pub struct Descriptor {
	pub addr:  u64,
	pub len:   u32,
	pub flags: u16,
	pub next:  u16,
}

#[repr(C)]
pub struct Available {
	pub flags: u16,
	pub idx:   u16,
	pub ring:  [u16; VIRTIO_RING_SIZE],
	pub event: u16,
}

#[repr(C)]
pub struct UsedElem {
	pub id:  u32,
	pub len: u32,
}

#[repr(C)]
pub struct Used {
	pub flags: u16,
	pub idx:   u16,
	pub ring:  [UsedElem; VIRTIO_RING_SIZE],
	pub event: u16,
}

#[repr(C)]
pub struct Queue {
	pub desc:  [Descriptor; VIRTIO_RING_SIZE],
	pub avail: Available,
	pub used:  Used,
}

#[repr(usize)]
pub enum MmioOffsets {
	MagicValue = 0x000,
	Version = 0x004,
	DeviceId = 0x008,
	VendorId = 0x00c,
	HostFeatures = 0x010,
	HostFeaturesSel = 0x014,
	GuestFeatures = 0x020,
	GuestFeaturesSel = 0x024,
	GuestPageSize = 0x028,
	QueueSel = 0x030,
	QueueNumMax = 0x034,
	QueueNum = 0x038,
	QueueAlign = 0x03c,
	QueuePfn = 0x040,
	QueueNotify = 0x050,
	InterruptStatus = 0x060,
	InterruptAck = 0x064,
	Status = 0x070,
	Config = 0x100,
}

#[repr(usize)]
pub enum DeviceTypes {
    None = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    Entropy = 4,
    Gpu = 16,
    Input = 18,
    Memory = 24,
}

impl MmioOffsets {
	pub fn val(self) -> usize {
		self as usize
	}

	pub fn scaled(self, scale: usize) -> usize {
		self.val() / scale
	}

	pub fn scale32(self) -> usize {
		self.scaled(4)
	}
}


pub enum StatusField {
    Acknowledge = 1,
    Driver = 2,
    Failed = 128,
    FeaturesOk = 8,
    DriverOk = 4,
    DeviceNeedsReset = 64,
}

impl StatusField {
    pub fn val(self) -> usize {
        self as usize
    }
    pub fn val32(self) -> u32 {
        self as u32
    }
    pub fn test(sf: u32, bit: StatusField) -> bool {
        sf & bit.val32() != 0
    }
    pub fn is_failed(sf: u32) -> bool {
        StatusField::test(sf, StatusField::Failed)
    }
    pub fn needs_reset(sf: u32) -> bool {
        StatusField::test(sf, StatusField::DeviceNeedsReset)
    }
    pub fn driver_ok(sf: u32) -> bool {
        StatusField::test(sf, StatusField::DriverOk)
    }
    pub fn features_ok(sf: u32) -> bool {
        StatusField::test(sf, StatusField::FeaturesOk)
    }
}

// We probably shouldn't put these here, but it'll help
// with probing the bus, etc. These are architecture specific
// which is why I say that.
pub const MMIO_VIRTIO_START: usize = 0x1000_1000;
pub const MMIO_VIRTIO_END: usize = 0x1000_8000;
pub const MMIO_VIRTIO_STRIDE: usize = 0x1000;
pub const MMIO_VIRTIO_MAGIC: u32 = 0x74_72_69_76;

pub struct VirtioDevice {
    pub devtype: DeviceTypes,
    pub valid: bool,
}

impl VirtioDevice {
    pub const fn new() -> Self {
        VirtioDevice {
            devtype: DeviceTypes::None,
            valid: false,
        }
    }
}

static mut VIRTIO_DEVICES: [VirtioDevice; 8] = [ 
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    VirtioDevice::new(),
    ];


/// Probe the VirtIO bus for devices that might be
/// out there.
pub fn probe() {
	// Rust's for loop uses an Iterator object, which now has a step_by
	// modifier to change how much it steps. Also recall that ..= means up
	// to AND including MMIO_VIRTIO_END.
	for addr in (MMIO_VIRTIO_START..=MMIO_VIRTIO_END).step_by(MMIO_VIRTIO_STRIDE) {
        print!("Virtio probing 0x{:08x}...", addr);
        let magicvalue;
        let deviceid;
        let ptr = addr as *mut u32;
        unsafe {
            magicvalue = ptr.read_volatile();
            deviceid = ptr.add(2).read_volatile();
        }
        // 0x74_72_69_76 is "virt" in little endian, so in reality
        // it is triv. All VirtIO devices have this attached to the
        // MagicValue register (offset 0x000)
        if MMIO_VIRTIO_MAGIC != magicvalue {
            println!("not virtio.");
        }
        // If we are a virtio device, we now need to see if anything
        // is actually attached to it. The DeviceID register will
        // contain what type of device this is. If this value is 0,
        // then it is not connected.
        else if 0 == deviceid {
            println!("not connected.");
        }
        // If we get here, we have a connected virtio device. Now we have
        // to figure out what kind it is so we can do device-specific setup.
        else {
            match deviceid {
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
                        let idx = (addr - MMIO_VIRTIO_START) >> 12;
                        unsafe {
                            VIRTIO_DEVICES[idx].devtype = DeviceTypes::Block;
                            VIRTIO_DEVICES[idx].valid = true;
                        }
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

pub fn handle_interrupt(interrupt: u32) {
    let idx = interrupt as usize - 1;
    unsafe {
        let ref vd = VIRTIO_DEVICES[idx];
        if false == vd.valid {
            println!("Spurious interrupt {}", interrupt);
        }
        else {
            match vd.devtype {
                DeviceTypes::Block => {
                    block::handle_interrupt(idx);
                },
                _ => {
                    println!("Invalid device generated interrupt!");
                }
            }
        }
    }
}
