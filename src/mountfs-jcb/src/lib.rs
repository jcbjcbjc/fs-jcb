#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;
#[macro_use]
extern crate log;
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
};
use core::{any::Any, future::Future, pin::Pin};
use fs_jcb::{FileSystem, FileType, Inode, MetaData, Result};
use rcore_fs::vfs::*;
use spin::{Mutex, RwLock};

pub type InodeId=usize;

pub struct MountFs{
    inner:Arc<dyn FileSystem>,
    mount_points:RwLock<BTreeMap<InodeId,Arc<MountFs>>>,
    self_mountpoint:Option<Arc<MNode>>,
    self_ref:Weak<MountFs>
}

impl FileSystem for MountFs{
    fn root_inode(&self) -> Arc<dyn Inode> {
        match &self.self_mountpoint{
            Some(inode)=>inode.fs.root_inode(),
            None=>self.mountpoint_root_inode()
        }
    }
}

impl MountFs{
    fn new()->Self{
        todo!()
    }

    fn wrap(self) -> Arc<Self> {
        // Create an Arc, make a Weak from it, then put it into the struct.
        // It's a little tricky.
        let inode = Arc::new(self);
        let weak = Arc::downgrade(&inode);
        let ptr = Arc::into_raw(inode) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
        }
    }
    /// return the root_inode of the fs
    fn mountpoint_root_inode(&self) ->Arc<MNode>{
        MNode{
            inner: self.inner.root_inode(),
            fs: self.self_ref.upgrade().clone()?,
            self_ref: Weak::default(),
        }.wrap()
    }
}
#[derive(Clone)]
pub struct MNode{
    inner:Arc<dyn Inode>,

    fs:Arc<MountFs>,
    self_ref:Weak<MNode>

}
impl MNode{

    /// Used in constructors.
    fn wrap(self) -> Arc<Self> {
        // Create an Arc, make a Weak from it, then put it into the struct.
        // It's a little tricky.
        let inode = Arc::new(self);
        let weak = Arc::downgrade(&inode);
        let ptr = Arc::into_raw(inode) as *mut Self;
        unsafe {
            (*ptr).self_ref = weak;
            Arc::from_raw(ptr)
        }
    }
    fn mount(&self,fs:Arc<dyn FileSystem>)->Result<Arc<MountFs>>{
        let mount_fs=MountFs{
            inner: fs,
            mount_points: RwLock::new(BTreeMap::new()),
            /// TODO FINXE
            self_mountpoint: Some(Arc::new(self.clone())),
            self_ref: Weak::default(),
        }.wrap();
        ///TODO wrap
        self.fs.mount_points.write()?.insert(self.inner.metadata()?.inode_id,mount_fs.clone());
        Ok(mount_fs)
    }

    fn change_inode(&self)->Option<Arc<MNode>>{
        if let Some(fs)=self.fs.mount_points.read().get(&self.inner.metadata()?.inode_id){
            Some(fs.mountpoint_root_inode())
        }else{
            Some(self.self_ref.upgrade()?.clone())
        }
    }
    fn is_mountpoint_root_inode(&self)->bool{
        self.fs.mountpoint_root_inode().metadata()?.inode_id==self.metadata()?.inode_id
    }
    fn find(&self,name:&str)->Result<Arc<MNode>>{
        match name{
            ".."=>{
               if self.is_mountpoint_root_inode(){
                   match &self.fs.self_mountpoint{
                       Some(inode)=>inode.inner.find(".."),
                       None=>self.self_ref.upgrade().clone()
                   }
               }else {
                   MNode{
                       inner:self.inner.find("..")?,
                       fs: self.fs.clone(),
                       self_ref: Weak::default(),
                   }.wrap()
               }
            }
            "."|""=>{
               Ok(self.self_ref.upgrade()?.clone())
            }
            _=>{
                let self_inode=self.change_inode()?;

                let m_node =MNode{
                    inner: Arc::new(self_inode.inner.find(name)?),
                    fs: self.fs.clone(),
                    self_ref: Default::default(),
                }.wrap();
                Ok(m_node)
            }
        }
    }
}

impl Inode for MNode{
    fn metadata(&self)->Result<MetaData>{
        Ok(())
    }
    fn set_metadata(&self)->Result<()>{
        Err(FsError::NotSupported)
    }
    fn create(&self, name: &str, type_: FileType, mode: u32) -> Option<Arc<dyn Inode>> {
        todo!()
    }

    fn find(&self, name: &str) -> Option<Arc<dyn Inode>> {




    }

    fn get_entry_with_meta_data(&self, entry_id: usize) -> Result<(fs_jcb::vfs::MetaData, String)> {
        todo!()
    }

    fn get_entry(&self, entry_id: usize) -> Result<String> {
        todo!()
    }

    fn read_at(&self,offset:usize,buf:&[u8])->Result<usize>{Err(FsError::NotSupported)}

    fn write_at(&self,offset:usize,buf:&mut [u8])->Result<usize>{Err(FsError::NotSupported)}

    fn resize(&self)->Result<()>{
        Err(FsError::NotSupported)
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        todo!()
    }
}





