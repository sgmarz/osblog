// fd.rs
// File descriptor routines and data structures
// Stephen Marz
// 11 Jun 2020

pub trait Descriptor {
    fn get_type(&self) -> DescriptorType;
}
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

#[derive(Copy, Clone)]
pub struct FileDescriptor {
    pub dtype: DescriptorType,

}

impl Descriptor for FileDescriptor {
    fn get_type(&self) -> DescriptorType {
        self.dtype
    }
}
