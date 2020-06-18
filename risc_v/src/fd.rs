// fd.rs
// File descriptor routines and data structures
// Stephen Marz
// 11 Jun 2020

use crate::vfs::Entry;

#[derive(Copy, Clone)]
pub enum DescriptorType {
	File(Entry),
	Device,
	Framebuffer,
	ButtonEvents,
	AbsoluteEvents,
	ConsoleIn,
	ConsoleOut,
	Network,
	Unknown,
}

