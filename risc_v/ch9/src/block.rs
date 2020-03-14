// block.rs
// Block device using VirtIO protocol
// Stephen Marz
// 10 March 2020

use crate::{kmem::{kfree, kmalloc},
            page::{zalloc, PAGE_SIZE},
            virtio,
            virtio::{Descriptor, MmioOffsets, Queue, StatusField, VIRTIO_RING_SIZE}};
use core::mem::size_of;

#[repr(C)]
pub struct Geometry {
	cylinders: u16,
	heads:     u8,
	sectors:   u8,
}

#[repr(C)]
pub struct Topology {
	physical_block_exp: u8,
	alignment_offset:   u8,
	min_io_size:        u16,
	opt_io_size:        u32,
}

// There is a configuration space for VirtIO that begins
// at offset 0x100 and continues to the size of the configuration.
// The structure below represents the configuration for a
// block device. Really, all that this OS cares about is the
// capacity.
#[repr(C)]
pub struct Config {
	capacity:                 u64,
	size_max:                 u32,
	seg_max:                  u32,
	geometry:                 Geometry,
	blk_size:                 u32,
	topology:                 Topology,
	writeback:                u8,
	unused0:                  [u8; 3],
	max_discard_sector:       u32,
	max_discard_seg:          u32,
	discard_sector_alignment: u32,
	max_write_zeroes_sectors: u32,
	max_write_zeroes_seg:     u32,
	write_zeroes_may_unmap:   u8,
	unused1:                  [u8; 3],
}

#[repr(C)]
pub struct Header {
	blktype:  u32,
	reserved: u32,
	sector:   u64,
}

#[repr(C)]
pub struct Data {
	data: *mut u8,
}

#[repr(C)]
pub struct Status {
	status: u8,
}

#[repr(C)]
pub struct Request {
	header: Header,
	data:   Data,
	status: Status,
	head:   u16,
}

// Internal block device structure
pub struct BlockDevice {
	queue:        *mut Queue,
	dev:          *mut u32,
	idx:          u16,
	ack_used_idx: u16,
	read_only:    bool,
}

// Type values
pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;
pub const VIRTIO_BLK_T_FLUSH: u32 = 4;
pub const VIRTIO_BLK_T_DISCARD: u32 = 11;
pub const VIRTIO_BLK_T_WRITE_ZEROES: u32 = 13;

// Status values
pub const VIRTIO_BLK_S_OK: u8 = 0;
pub const VIRTIO_BLK_S_IOERR: u8 = 1;
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2;

// Feature bits
pub const VIRTIO_BLK_F_SIZE_MAX: u32 = 1;
pub const VIRTIO_BLK_F_SEG_MAX: u32 = 2;
pub const VIRTIO_BLK_F_GEOMETRY: u32 = 4;
pub const VIRTIO_BLK_F_RO: u32 = 5;
pub const VIRTIO_BLK_F_BLK_SIZE: u32 = 6;
pub const VIRTIO_BLK_F_FLUSH: u32 = 9;
pub const VIRTIO_BLK_F_TOPOLOGY: u32 = 10;
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11;
pub const VIRTIO_BLK_F_DISCARD: u32 = 13;
pub const VIRTIO_BLK_F_WRITE_ZEROES: u32 = 14;

// Much like with processes, Rust requires some initialization
// when we declare a static. In this case, we use the Option
// value type to signal that the variable exists, but not the
// queue itself. We will replace this with an actual queue when
// we initialize the block system.
static mut BLOCK_DEVICES: [Option<BlockDevice>; 8] = [None, None, None, None, None, None, None, None];

pub fn setup_block_device(ptr: *mut u32) -> bool {
	unsafe {
		// We can get the index of the device based on its address.
		// 0x1000_1000 is index 0
		// 0x1000_2000 is index 1
		// ...
		// 0x1000_8000 is index 7
		// To get the number that changes over, we shift right 12 places (3 hex digits)
		let idx = (ptr as usize - virtio::MMIO_VIRTIO_START) >> 12;
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
		let host_features = ptr.add(MmioOffsets::HostFeatures.scale32()).read_volatile();
		let guest_features = host_features & !(1 << VIRTIO_BLK_F_RO);
		let ro = host_features & (1 << VIRTIO_BLK_F_RO) != 0;
		ptr.add(MmioOffsets::GuestFeatures.scale32()).write_volatile(guest_features);
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
		let queue_ptr = zalloc(num_pages) as *mut Queue;
		let queue_pfn = queue_ptr as u32;
		ptr.add(MmioOffsets::GuestPageSize.scale32()).write_volatile(PAGE_SIZE as u32);
		// QueuePFN is a physical page number, however it
		// appears for QEMU we have to write the entire memory
		// address. This is a physical memory address where we
		// (the OS) and the block device have in common for
		// making and receiving requests.
		ptr.add(MmioOffsets::QueuePfn.scale32()).write_volatile(queue_pfn / PAGE_SIZE as u32);
		// We need to store all of this data as a "BlockDevice"
		// structure We will be referring to this structure when
		// making block requests AND when handling responses.
		let bd = BlockDevice { queue:        queue_ptr,
		                       dev:          ptr,
		                       idx:          0,
		                       ack_used_idx: 0,
		                       read_only:    ro, };
		BLOCK_DEVICES[idx] = Some(bd);

		// 8. Set the DRIVER_OK status bit. Device is now "live"
		status_bits |= StatusField::DriverOk.val32();
		ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);

		true
	}
}

pub fn fill_next_descriptor(bd: &mut BlockDevice, desc: Descriptor) -> u16 {
	unsafe {
		bd.idx = (bd.idx + 1) % VIRTIO_RING_SIZE as u16;
		(*bd.queue).desc[bd.idx as usize] = desc;
		if (*bd.queue).desc[bd.idx as usize].flags & virtio::VIRTIO_DESC_F_NEXT != 0 {
			(*bd.queue).desc[bd.idx as usize].next = (bd.idx + 1) % VIRTIO_RING_SIZE as u16;
		}
		bd.idx
	}
}

pub fn block_op(dev: usize, buffer: *mut u8, size: u32, offset: u64, write: bool) {
	unsafe {
		if let Some(bdev) = BLOCK_DEVICES[dev - 1].as_mut() {
			// Check to see if we are trying to write to a read only device.
			if true == bdev.read_only && true == write {
				println!("Trying to write to read/only!");
				return;
			}
			let sector = offset / 512;
			let blk_request_size = size_of::<Request>();
			let blk_request = kmalloc(blk_request_size) as *mut Request;
			let desc = Descriptor { addr:  &(*blk_request).header as *const Header as u64,
			                        len:   size_of::<Header>() as u32,
			                        flags: virtio::VIRTIO_DESC_F_NEXT,
			                        next:  0, };
			let head_idx = fill_next_descriptor(bdev, desc);
			(*blk_request).header.sector = sector;
			// A write is an "out" direction, whereas a read is an "in" direction.
			(*blk_request).header.blktype = if true == write {
				VIRTIO_BLK_T_OUT
			}
			else {
				VIRTIO_BLK_T_IN
			};
			// We put 111 in the status. Whenever the device finishes, it will write into
			// status. If we read status and it is 111, we know that it wasn't written to by
			// the device.
			(*blk_request).data.data = buffer;
			(*blk_request).header.reserved = 0;
			(*blk_request).status.status = 111;
			let desc = Descriptor { addr:  buffer as u64,
			                        len:   size,
			                        flags: virtio::VIRTIO_DESC_F_NEXT
			                               | if false == write {
				                               virtio::VIRTIO_DESC_F_WRITE
			                               }
			                               else {
				                               0
			                               },
			                        next:  0, };
			let _data_idx = fill_next_descriptor(bdev, desc);
			let desc = Descriptor { addr:  &(*blk_request).status as *const Status as u64,
			                        len:   size_of::<Status>() as u32,
			                        flags: virtio::VIRTIO_DESC_F_WRITE,
			                        next:  0, };
			let _status_idx = fill_next_descriptor(bdev, desc);
			(*bdev.queue).avail.ring[(*bdev.queue).avail.idx as usize] = head_idx;
			(*bdev.queue).avail.idx = ((*bdev.queue).avail.idx + 1) % virtio::VIRTIO_RING_SIZE as u16;
			// The only queue a block device has is 0, which is the request
			// queue.
			bdev.dev.add(MmioOffsets::QueueNotify.scale32()).write_volatile(0);
		}
	}
}

pub fn read(dev: usize, buffer: *mut u8, size: u32, offset: u64) {
	block_op(dev, buffer, size, offset, false);
}

pub fn write(dev: usize, buffer: *mut u8, size: u32, offset: u64) {
	block_op(dev, buffer, size, offset, true);
}

/// Here we handle block specific interrupts. Here, we need to check
/// the used ring and wind it up until we've handled everything.
/// This is how the device tells us that it's finished a request.
pub fn pending(bd: &mut BlockDevice) {
	// Here we need to check the used ring and then free the resources
	// given by the descriptor id.
	unsafe {
		let ref queue = *bd.queue;
		while bd.ack_used_idx != queue.used.idx {
			let ref elem = queue.used.ring[bd.ack_used_idx as usize];
			bd.ack_used_idx = (bd.ack_used_idx + 1) % VIRTIO_RING_SIZE as u16;
			let rq = queue.desc[elem.id as usize].addr as *const Request;
			kfree(rq as *mut u8);
			// TODO: Awaken the process that will need this I/O. This is
			// the purpose of the waiting state.
		}
	}
}

/// The trap code will route PLIC interrupts 1..=8 for virtio devices. When
/// virtio determines that this is a block device, it sends it here.
pub fn handle_interrupt(idx: usize) {
	unsafe {
		if let Some(bdev) = BLOCK_DEVICES[idx].as_mut() {
			pending(bdev);
		}
		else {
			println!("Invalid block device for interrupt {}", idx + 1);
		}
	}
}
