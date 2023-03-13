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
use fs_jcb::{FileSystem, FileType, Inode, Result};
use rcore_fs::vfs::*;
use spin::RwLock;

pub type InodeId=usize;

pub struct MountFs{
    inner:Arc<dyn FileSystem>,
    mount_points:RwLock<BTreeMap<InodeId,Arc<MountFs>>>,
    self_mountpoint:Option<Arc<MNode>>,
    self_ref:Weak<MountFs>
}

impl FileSystem for MountFs{
    fn root_inode(&self) -> Arc<dyn Inode> {
        todo!()
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
    /// Wrap pure `INode` with `Arc<..>`.
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

    fn mount_root_inode(&self)->Option<Arc<MNode>>{
        if let Some(fs)=self.fs.mount_points.read().get(&self.inner.metadata()?.inode_id){
            Some(fs.root_inode())
        }else{
            Some(self.fs.mountpoint_root_inode())
        }
    }
    fn is_mount_root_inode(&self)->bool{
        self.fs.root_inode().metadata()?.inode_id==self.metadata()?
    }
    fn find(&self,name:&str)->Result<Arc<MNode>>{
        match name{
            ".."=>{
                let inner=self.inner.find(name)?;

                if inner.metadata()?.inode_id==self.fs.inner.root_inode().metadata()?.inode_id{
                    Ok(self.fs.root_inode())
                }else{
                    Ok(inner)
                }
            }
            "."|""=>{
               Ok(self.self_ref.upgrade()?.clone())
            }
            _=>{
                let self_inode=self.mount_root_inode()?;

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





