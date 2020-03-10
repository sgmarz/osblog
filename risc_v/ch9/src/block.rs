// block.rs
// Block device using VirtIO protocol
// Stephen Marz
// 10 March 2020

use crate::{page::{zalloc, PAGE_SIZE},
            virtio::{MmioOffsets, Queue}};
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
		// The following can get dangerous for multi-harts. For now, we
		// only have a single hart, so we can assume there is no race
		// condition when creating the BDA.
		if let Some(mut vdq) = BLOCK_DEVICE_ARRAY.take() {
			// First, if the block device array is empty, create it!
			// We add 4096 to round this up. Usually this queue
			// comes out to be 6.5 pages, so we increase this to get
			// 7 pages.
			let num_pages =
				(size_of::<Queue>() + PAGE_SIZE) / PAGE_SIZE;
			let queue_ptr = zalloc(num_pages) as *mut Queue;
			let queue_pfn = queue_ptr as u32;
			// let config_ptr =
			// ptr.add(MmioOffsets::Config.scale32()) as
			// *const Config; let ref config = *config_ptr;
            let bd = BlockDevice { queue: queue_ptr,
                                   dev: ptr,
			                       idx:   1, };
			vdq.push_back(bd);
			ptr.add(MmioOffsets::QueuePfn.scale32())
			   .write_volatile(queue_pfn);
			BLOCK_DEVICE_ARRAY.replace(vdq);
			true
		}
		else {
			false
		}
	}
}
