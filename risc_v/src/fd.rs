// fd.rs
// File descriptor routines and data structures
// Stephen Marz
// 11 Jun 2020

#[derive(Copy, Clone)]
pub enum DescriptorType {
	File,
	Device,
	Framebuffer,
	ButtonEvents,
	AbsoluteEvents,
	ConsoleIn,
	ConsoleOut,
	Network,
	Unknown,
}

