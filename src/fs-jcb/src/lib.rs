#![feature(linked_list_remove)]
#![feature(slice_pattern)]
//!An easy file system isolated from the kernel
#![no_std]
#![no_std]
#![deny(missing_docs)]

extern crate alloc;
mod block_device;
mod vfs;
mod util;
pub use vfs::{Inode,FileSystem,Result,FileType,MetaData,De};
pub use block_device::{BlockDevice,Device};
pub use util::{BlockIter,BlockRange,Dirty,uninit_memory};




