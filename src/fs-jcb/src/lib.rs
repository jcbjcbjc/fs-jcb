#![feature(linked_list_remove)]
//!An easy file system isolated from the kernel
#![no_std]
#![no_std]
#![deny(missing_docs)]

extern crate alloc;
mod block_device;
mod vfs;
mod util;
pub use vfs::{Inode,FileSystem,Result,FileType,MetaData};
pub use block_device::BlockDevice;




