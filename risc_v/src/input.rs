// input.rs
// Input handling.
// Stephen Marz

use crate::virtio::{Queue, MmioOffsets, MMIO_VIRTIO_START, StatusField, VIRTIO_RING_SIZE, Descriptor, VIRTIO_DESC_F_WRITE, VIRTIO_F_RING_EVENT_IDX};
use crate::kmem::kmalloc;
use crate::page::{PAGE_SIZE, zalloc};
use core::mem::size_of;
use alloc::collections::VecDeque;

pub static mut ABS_EVENTS: Option<VecDeque<Event>> = None;
pub static mut ABS_OBSERVERS: Option<VecDeque<u16>> = None;
pub static mut KEY_EVENTS: Option<VecDeque<Event>> = None;
pub static mut KEY_OBSERVERS: Option<VecDeque<u16>> = None;

const EVENT_BUFFER_ELEMENTS: usize = 64;

pub enum InputType {
	None,
	Abs(u32, u32, u32, u32, u32),
	Key(u32, u32)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub code: u16,
    pub value: u32,
}
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ConfigSelect {
    UNSET = 0x00,
    IdName = 0x01,
    IdSerial = 0x02,
    IdDevids = 0x03,
    PropBits = 0x10,
    EvBits = 0x11,
    AbsInfo = 0x12,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AbsInfo {
    pub min: u32,
    pub max: u32,
    pub fuzz: u32,
    pub flat: u32,
    pub res: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DevIds {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ConfigUnion {
    pub string: [u8; 128],
    pub bitmap: [i8; 128],
    pub abs: AbsInfo,
    pub ids: DevIds,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Config {
    pub select: ConfigSelect,
    pub subsel: u8,
    pub size: u8,
    reserved: [u8; 5],
    pub config: ConfigUnion,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum EventType {
    Syn = 0x00,
    Key = 0x01,
    Rel = 0x02,
    Abs = 0x03,
    Msc = 0x04,
    Sw = 0x05,
    Led = 0x11,
    Snd = 0x12,
    Rep = 0x14,
    Ff = 0x15,
    Pwr = 0x16,
    FfStatus = 0x17,
    Max = 0x1f,
}

const EVENT_SIZE: usize = size_of::<Event>();

pub struct Device {
	event_queue:  *mut Queue,
	status_queue: *mut Queue,  
	dev:          *mut u32,
	event_idx:          u16,
	event_ack_used_idx: u16,
	event_buffer: *mut Event,
	status_idx:          u16,
	status_ack_used_idx: u16,
	status_buffer: *mut Event,
}

pub static mut INPUT_DEVICES: [Option<Device>; 8] = [
	None,
	None,
	None,
	None,
	None,
	None,
	None,
	None,
];

pub fn setup_input_device(ptr: *mut u32) -> bool {
	unsafe {
		// We can get the index of the device based on its address.
		// 0x1000_1000 is index 0
		// 0x1000_2000 is index 1
		// ...
		// 0x1000_8000 is index 7
		// To get the number that changes over, we shift right 12 places (3 hex digits)
		let idx = (ptr as usize - MMIO_VIRTIO_START) >> 12;
		// [Driver] Device Initialization
		// 1. Reset the device (write 0 into status)
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(0);
		let mut status_bits = StatusField::Acknowledge.val32();
		// 2. Set ACKNOWLEDGE status bit
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
		// 3. Set the DRIVER status bit
		status_bits |= StatusField::DriverOk.val32();
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
		// 4. Read device feature bits, write subset of feature
		// bits understood by OS and driver    to the device.
		let mut host_features = ptr.add(MmioOffsets::HostFeatures.scale32()).read_volatile();
		// Turn off EVENT_IDX
		host_features &= !(1 << VIRTIO_F_RING_EVENT_IDX);
		ptr.add(MmioOffsets::GuestFeatures.scale32()).write_volatile(host_features);
		// 5. Set the FEATURES_OK status bit
		status_bits |= StatusField::FeaturesOk.val32();
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
		// 6. Re-read status to ensure FEATURES_OK is still set.
		// Otherwise, it doesn't support our features.
		let status_ok = ptr.add(MmioOffsets::Status.scale32()).read_volatile();
		// If the status field no longer has features_ok set,
		// that means that the device couldn't accept
		// the features that we request. Therefore, this is
		// considered a "failed" state.
		if false == StatusField::features_ok(status_ok) {
			print!("features fail...");
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(StatusField::Failed.val32());
			return false;
		}
		// 7. Perform device-specific setup.
		// Set the queue num. We have to make sure that the
		// queue size is valid because the device can only take
		// a certain size.
		let qnmax = ptr.add(MmioOffsets::QueueNumMax.scale32()).read_volatile();
		ptr.add(MmioOffsets::QueueNum.scale32()).write_volatile(VIRTIO_RING_SIZE as u32);
		if VIRTIO_RING_SIZE as u32 > qnmax {
			print!("queue size fail...");
			return false;
		}
		// First, if the block device array is empty, create it!
		// We add 4095 to round this up and then do an integer
		// divide to truncate the decimal. We don't add 4096,
		// because if it is exactly 4096 bytes, we would get two
		// pages, not one.
		let num_pages = (size_of::<Queue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		// println!("np = {}", num_pages);
		// We allocate a page for each device. This will the the
		// descriptor where we can communicate with the block
		// device. We will still use an MMIO register (in
		// particular, QueueNotify) to actually tell the device
		// we put something in memory. We also have to be
		// careful with memory ordering. We don't want to
		// issue a notify before all memory writes have
		// finished. We will look at that later, but we need
		// what is called a memory "fence" or barrier.
		ptr.add(MmioOffsets::QueueSel.scale32()).write_volatile(0);

		// Alignment is very important here. This is the memory address
		// alignment between the available and used rings. If this is wrong,
		// then we and the device will refer to different memory addresses
		// and hence get the wrong data in the used ring.
		// ptr.add(MmioOffsets::QueueAlign.scale32()).write_volatile(2);
		let event_queue_ptr = zalloc(num_pages) as *mut Queue;
		let queue_pfn = event_queue_ptr as u32;
		ptr.add(MmioOffsets::GuestPageSize.scale32()).write_volatile(PAGE_SIZE as u32);
		ptr.add(MmioOffsets::QueuePfn.scale32()).write_volatile(queue_pfn / PAGE_SIZE as u32);
		// Status queue
		ptr.add(MmioOffsets::QueueSel.scale32()).write_volatile(1);
		// Alignment is very important here. This is the memory address
		// alignment between the available and used rings. If this is wrong,
		// then we and the device will refer to different memory addresses
		// and hence get the wrong data in the used ring.
		// ptr.add(MmioOffsets::QueueAlign.scale32()).write_volatile(2);
		let status_queue_ptr = zalloc(num_pages) as *mut Queue;
		let queue_pfn = status_queue_ptr as u32;
		ptr.add(MmioOffsets::GuestPageSize.scale32()).write_volatile(PAGE_SIZE as u32);
		ptr.add(MmioOffsets::QueuePfn.scale32()).write_volatile(queue_pfn / PAGE_SIZE as u32);
		// 8. Set the DRIVER_OK status bit. Device is now "live"
		status_bits |= StatusField::DriverOk.val32();
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);

        let config_ptr = ptr.add(MmioOffsets::Config.scale32()) as *mut Config;

        // let mut config = config_ptr.read_volatile();

        // config.select = ConfigSelect::AbsInfo;
        // config.subsel = 0;

        // config_ptr.write_volatile(config);
		// let id = config_ptr.read_volatile().config.abs;
		// println!("Min: {}, Max: {}, fuzz: {}, flat: {}, res: {}", id.min, id.max, id.fuzz, id.flat, id.res);

		let mut dev = Device {
			event_queue: event_queue_ptr,
			status_queue: status_queue_ptr,
			dev: ptr,
			status_idx: 0,
			status_ack_used_idx: 0,
			status_buffer: kmalloc(EVENT_SIZE * EVENT_BUFFER_ELEMENTS) as *mut Event,
			event_idx: 0,
			event_ack_used_idx: 0,
			event_buffer: kmalloc(EVENT_SIZE * EVENT_BUFFER_ELEMENTS) as *mut Event,
		};
		for i in 0..EVENT_BUFFER_ELEMENTS {
			repopulate_event(&mut dev, i);
		}
		INPUT_DEVICES[idx] = Some(dev);
		ABS_EVENTS = Some(VecDeque::with_capacity(1000));
		// ABS_OBSERVERS = Some(VecDeque::new());
		KEY_EVENTS = Some(VecDeque::with_capacity(1000));
		// KEY_OBSERVERS = Some(VecDeque::new());

		true
	}
}

unsafe fn repopulate_event(dev: &mut Device, buffer: usize) {
// Populate eventq with buffers, these must be at least the size of struct virtio_input_event.
	let desc = Descriptor {
		addr: dev.event_buffer.add(buffer) as u64,
		len: EVENT_SIZE as u32,
		flags: VIRTIO_DESC_F_WRITE,
		next: 0
	};
	let head = dev.event_idx as u16;
	(*dev.event_queue).desc[dev.event_idx as usize] = desc;
	dev.event_idx = (dev.event_idx + 1) % VIRTIO_RING_SIZE as u16;
	(*dev.event_queue).avail.ring[(*dev.event_queue).avail.idx as usize % VIRTIO_RING_SIZE] = head;
	(*dev.event_queue).avail.idx = (*dev.event_queue).avail.idx.wrapping_add(1);
}

fn pending(dev: &mut Device) {
	// Here we need to check the used ring and then free the resources
	// given by the descriptor id.
	unsafe {
		// Check the event queue first
		let ref queue = *dev.event_queue;
		while dev.event_ack_used_idx != queue.used.idx {
			let ref elem = queue.used.ring[dev.event_ack_used_idx as usize % VIRTIO_RING_SIZE];
			let ref desc = queue.desc[elem.id as usize];
			let event = (desc.addr as *const Event).as_ref().unwrap();
			// print!("EAck {}, elem {}, len {}, addr 0x{:08x}: ", dev.event_ack_used_idx, elem.id, elem.len, desc.addr as usize);
			// println!("Type = {:x}, Code = {:x}, Value = {:x}", event.event_type, event.code, event.value);
			repopulate_event(dev, elem.id as usize);
			dev.event_ack_used_idx = dev.event_ack_used_idx.wrapping_add(1);
			match event.event_type {
				EventType::Abs => {
					let mut ev = ABS_EVENTS.take().unwrap();
					ev.push_back(*event);
					ABS_EVENTS.replace(ev);	
				},
				EventType::Key => {
					let mut ev = KEY_EVENTS.take().unwrap();
					ev.push_back(*event);
					KEY_EVENTS.replace(ev);	
				},
				_ => {

				}
			}
		}
		// Next, the status queue
		let ref queue = *dev.status_queue;
		while dev.status_ack_used_idx != queue.used.idx {
			let ref elem = queue.used.ring[dev.status_ack_used_idx as usize % VIRTIO_RING_SIZE];
			print!("SAck {}, elem {}, len {}: ", dev.status_ack_used_idx, elem.id, elem.len);
			let ref desc = queue.desc[elem.id as usize];
			let event = (desc.addr as *const Event).as_ref().unwrap();
			println!("Type = {:x}, Code = {:x}, Value = {:x}", event.event_type as u8, event.code, event.value);
			dev.status_ack_used_idx = dev.status_ack_used_idx.wrapping_add(1);
		}
	}
}

pub fn handle_interrupt(idx: usize) {
	unsafe {
		if let Some(bdev) = INPUT_DEVICES[idx].as_mut() {
			pending(bdev);
		}
		else {
			println!(
			         "Invalid input device for interrupt {}",
			         idx + 1
			);
		}
	}
}

