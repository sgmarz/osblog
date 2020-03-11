// block.rs
// Block device using VirtIO protocol
// Stephen Marz
// 10 March 2020

use crate::{page::{zalloc, PAGE_SIZE},
            virtio::{MmioOffsets, Queue, VIRTIO_RING_SIZE, StatusField}};
use alloc::collections::VecDeque;
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

// Internal block device structure
pub struct BlockDevice {
    queue: *mut Queue,
    dev: *mut u32,
	idx:   u16,
}

static mut BLOCK_DEVICE_ARRAY: Option<VecDeque<BlockDevice>> = None;

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

pub fn init_block_system() {
	unsafe {
		BLOCK_DEVICE_ARRAY.replace(VecDeque::with_capacity(1));
	}
}

pub fn setup_block_device(ptr: *mut u32) -> bool {
	unsafe {
		if let Some(mut vdq) = BLOCK_DEVICE_ARRAY.take() {
			// [Driver] Device Initialization
			// 1. Reset the device (write 0 into status)
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(0);
			let mut status_bits = StatusField::Acknowledge.val32();
			// 2. Set ACKNOWLEDGE status bit
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
			// 3. Set the DRIVER status bit
			status_bits |= StatusField::DriverOk.val32();
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
			// 4. Read device feature bits, write subset of feature bits understood by OS and driver
			//    to the device.
			let host_features = ptr.add(MmioOffsets::HostFeatures.scale32()).read_volatile();
			let guest_features = host_features & !(1 << VIRTIO_BLK_F_RO);
			ptr.add(MmioOffsets::GuestFeatures.scale32()).write_volatile(guest_features);
			// 5. Set the FEATURES_OK status bit
			status_bits |= StatusField::FeaturesOk.val32();
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
			// 6. Re-read status to ensure FEATURES_OK is still set. Otherwise, it doesn't support our features.
			let status_ok = ptr.add(MmioOffsets::Status.scale32()).read_volatile();
			// If the status field no longer has features_ok set, that means that the device couldn't accept
			// the features that we request. Therefore, this is considered a "failed" state.
			if false == StatusField::features_ok(status_ok) {
				print!("features fail...");
				ptr.add(MmioOffsets::Status.scale32()).write_volatile(StatusField::Failed.val32());
				return false;
			}
			// 7. Perform device-specific setup.
			// First, if the block device array is empty, create it!
			// We add 4096 to round this up and then do an integer divide
			// to truncate the decimal.
			let num_pages =
				(size_of::<Queue>() + PAGE_SIZE) / PAGE_SIZE;
			let queue_ptr = zalloc(num_pages) as *mut Queue;
			let queue_pfn = queue_ptr as u32;
            let bd = BlockDevice { queue: queue_ptr,
                                   dev: ptr,
			                       idx:   1, };
			vdq.push_back(bd);
			ptr.add(MmioOffsets::QueuePfn.scale32())
			   .write_volatile(queue_pfn);
			BLOCK_DEVICE_ARRAY.replace(vdq);

			ptr.add(MmioOffsets::QueueNum.scale32()).write_volatile(VIRTIO_RING_SIZE as u32);
			let qn = ptr.add(MmioOffsets::QueueNum.scale32()).read_volatile();
			let qnmax = ptr.add(MmioOffsets::QueueNumMax.scale32()).read_volatile();
			// 8. Set the DRIVER_OK status bit. Device is now "live"
			status_bits |= StatusField::DriverOk.val32();
			ptr.add(MmioOffsets::Status.scale32()).write_volatile(status_bits);
			true
		}
		else {
			false
		}
	}
}
