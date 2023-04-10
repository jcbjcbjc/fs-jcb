
#![no_std]
#![no_main]

extern crate alloc;

mod inode_impl;
mod structs;

use core::collections::BTreeMap;
use core::sync::Weak;
use core::sync::Arc;
use fs_jcb::{FileSystem, Inode};
use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use spin::{Mutex, RwLock};
use crate::bfs::inode_impl::InodeImpl;

use crate::bfs::structs::{Alloc, AsBuf, BLKSIZE, BlockId, DiskINode, FreeMap, InodeId, SuperBlock};
use crate::block_device::{BlockDevice, Device};
use crate::inode_impl::InodeImpl;
use crate::structs::{AsBuf, BLKSIZE, BlockId, DiskINode, FreeMap, InodeId, SuperBlock};
use crate::util::{Dirty, uninit_memory};
use crate::vfs;
use crate::vfs::{FileSystem, FsError, Inode};

trait DeviceExt: Device {
    fn read_block(&self, id: BlockId, offset: usize, buf: &mut [u8]) -> vfs::Result<()> {
        debug_assert!(offset + buf.len() <= BLKSIZE);
        match self.read_at(id * BLKSIZE + offset, buf) {
            Ok(len) if len == buf.len() => Ok(()),
            _ => panic!("cannot read block {} offset {} from device", id, offset),
        }
    }
    fn write_block(&self, id: BlockId, offset: usize, buf: &[u8]) -> vfs::Result<()> {
        debug_assert!(offset + buf.len() <= BLKSIZE);
        match self.write_at(id * BLKSIZE + offset, buf) {
            Ok(len) if len == buf.len() => Ok(()),
            _ => panic!("cannot write block {} offset {} to device", id, offset),
        }
    }
    /// Load struct `T` from given block in device
    /// TODO THINK ABOUT IT
    fn load_struct<T: AsBuf>(&self, id: BlockId) -> vfs::Result<T> {
        let mut s: T = unsafe { uninit_memory() };
        self.read_block(id, 0,  s.as_buf_mut())?;
        Ok(s)
    }
}

impl DeviceExt for dyn Device {}


pub struct JCBFileSystem{
    pub device:Arc<dyn Device>,

    cache_inodes: RwLock<BTreeMap<InodeId, Weak<InodeImpl>>>,

    free_map:RwLock<FreeMap>,

    super_block:RwLock<Dirty<SuperBlock>>
}

impl FileSystem for JCBFileSystem{

    fn root_inode(&self) -> Arc<dyn Inode> {
        None
    }
}
impl JCBFileSystem{
    pub fn create(
        block_device:Arc<dyn Device>,

    ) ->Arc<Mutex<Self>>{

    }

    pub fn open(
        block_device:Arc<dyn Device>
    )->Arc<Mutex<Self>>{


    }

    pub fn alloc_block(&self)->Option<BlockId>{
        let mut free_map=self.free_map.write();
        if let Some(id)=free_map.alloc(){
            let mut super_block=self.super_block.write();
            if super_block.unused_blocks == 0 {
                free_map[id]=true;
                return None;
            }
            super_block.unused_blocks-=1;
            Some(id)
        }else{
            /// TODO EXCEPTION


            None
        }
    }

    pub fn get_inode(&self,inode_id:InodeId)->Arc<InodeImpl>{
        if let Some(inode)=self.cache_inodes.read().get(&inode_id){
            if let Some(inode)=inode.upgrade(){
                return inode
            }
        }
        // func get_inode only can be called just after getting the inodeId determined in the disk,so the inode must exist
        // no inode or no Arc
        let disk_inode = Dirty::new(self.device.load_struct::<DiskINode>(inode_id).unwrap());
        self._new_inode(inode_id,disk_inode)
    }

    pub fn new_inode_file(&self)->vfs::Result<Arc<InodeImpl>>{
        let id=self.alloc_block().ok_or(FsError::NoDeviceSpace)?;
        let inode=self._new_inode(id,Dirty::new_dirty(DiskINode::new_file()));
        Ok(inode)
    }
    pub fn new_inode_dir(&self,parent:InodeId)->vfs::Result<Arc<InodeImpl>>{
        let id=self.alloc_block().ok_or(FsError::NoDeviceSpace)?;
        let inode=self._new_inode(id,Dirty::new_dirty(DiskINode::new_dir()));
        inode.init_dir_entry(parent);
        Ok(inode)
    }

    pub fn _new_inode(&self,id:InodeId,disk_inode:Dirty<DiskINode>)->Arc<InodeImpl>{
        let device_inode_id = disk_inode.device_inode_id;

        let inode=Arc::new(InodeImpl::new(
            id,
            RwLock::new(disk_inode),
            self.self_ptr.upgrade().unwrap(),
            device_inode_id
        ));
        self.cache_inodes.write().insert(id,Arc::downgrade(&inode));
        inode
    }
}

