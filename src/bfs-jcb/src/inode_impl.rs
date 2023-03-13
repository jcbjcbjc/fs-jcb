use std::collections::BTreeMap;
use std::fs::FileType;
use std::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use spin::RwLock;
use crate::bfs::{JCBFileSystem};
use crate::{Inode, JCBFileSystem, vfs};
use crate::bfs::structs::{AsBuf, DIRENT_SIZE, DiskEntry, DiskINode, InodeId, Str256};
use crate::structs::{DIRENT_SIZE, DiskEntry, DiskINode, InodeId};
use crate::util::{Dirty, uninit_memory};
use crate::vfs::{FileSystem, FileType, FsError, MetaData};


pub struct InodeImpl{
    /// INode number
    id: InodeId,
    /// On-disk INode
    disk_inode: RwLock<Dirty<DiskINode>>,
    /// Reference to SFS, used by almost all operations
    fs: Arc<JCBFileSystem>,
    /// Char/block device id (major, minor)
    /// e.g. crw-rw-rw- 1 root wheel 3, 2 May 13 16:40 /dev/null
    device_inode_id: usize,
    //cache
    cache_entrys:RwLock<BTreeMap<usize,DiskEntry>>
}
impl Inode for InodeImpl{
    fn metadata(&self)->Result<MetaData>{
        let inode=self.disk_inode.read();
        let meta_data=MetaData{
              
        };
        Ok(meta_data)
    }

    fn set_metadata(&self)->Result<()>{
        let inode=self.disk_inode.write();
        

    }

    fn create(&self, name: &str, type_: FileType,mode: u32) -> vfs::Result<Arc<dyn Inode>> {
        let mut inode=match type_ {
            vfs::FileType::File => self.fs.new_inode_file()?,

            vfs::FileType::Dir => self.fs.new_inode_dir(self.id)?,

            _ => return Err(vfs::FsError::InvalidParam),
        };

        self._append_dir_entry(&DiskEntry{
            inode_id:inode.id ,
            name: Str256::from(name)
        });

        Ok(inode)
    }
    fn find(&self, name: &str) ->vfs::Result<Arc<dyn Inode>> {
        let id=self.get_entry_and_inode_id(name).ok_or(FsError::EntryNotFound)?.0;
        let inode= self.fs.get_inode(id);
        Ok(inode)
    }
    fn get_entry_with_meta_data(&self, entry_id: usize) -> vfs::Result<(MetaData, String)> {
        if self.metadata()?.type_!=FileType::Dir{return Err(FsError::NotDir);}
        let disk_entry=self.read_dir_entry(entry_id).map_err(FsError::EntryNotFound)?;
        let meta_data=self.fs.get_inode(entry_id).metadata()?;
        Ok ((meta_data,String::from(disk_entry.name.as_ref())))
    }

    fn get_entry(&self, entry_id: usize) -> vfs::Result<String> {
        if self.disk_inode.read().type_ != FileType::Dir {
            return Err(FsError::NotDir);
        }
        if entry_id >= self.disk_inode.read().size as usize / DIRENT_SIZE {
            return Err(FsError::EntryNotFound);
        };
        let name=self.read_dir_entry(entry_id)?.name;
        Ok(String::from(name.as_ref()))
    }

    fn read_at(&self,offset:usize,buf:&mut [u8])->Result<usize>{Err(FsError::NotSupported)}

    fn write_at(&self,offset:usize,buf:&[u8])->Result<usize>{Err(FsError::NotSupported)}

    fn resize(&self)->Result<()>{
        Err(FsError::NotSupported)
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.fs.clone()
    }
}


impl InodeImpl{
    pub fn new(id: InodeId,
           disk_inode: RwLock<Dirty<DiskINode>>,
           fs: Arc<JCBFileSystem>,
               device_inode_id: usize) ->Self{
        InodeImpl{
            id,
            disk_inode,
            fs,
            device_inode_id,
            cache_entrys: RwLock::new(BTreeMap::new())
        }
    }
    fn get_entry_and_inode_id(&self,name:&str)->Option<(InodeId,usize)>{
        (0..self.disk_inode.read().size as usize / DIRENT_SIZE)
            .map(|i| (self._read_dir_entry(i as usize).unwrap(), i))
            .find(|(entry, _)| entry.name.as_ref() == name)
            .map(|(entry, id)| (entry.inode_id as InodeId, id as usize))
    }

    pub fn read_dir_entry(&self,entry_id:usize)->vfs::Result<DiskEntry>{
        if let Some(disk_entry)=self.cache_entrys.read().get(&entry_id){
            // TODO FIXME
            return Ok((*disk_entry).Clone())
        }
        self._read_dir_entry(entry_id)
    }

    pub fn init_dir_entry(&self, parent:InodeId){



    }
    pub fn write_dir_entry(&self,id:usize,disk_entry:& DiskEntry)->vfs::Result<()>{
        self.cache_entrys.write().insert(id, (*disk_entry).clone());

        self._write_dir_entry(id,disk_entry)
    }
    pub fn remove_dir_entry(&self,id:usize){
        self.cache_entrys.write().remove(&id);
        self._remove_dir_entry(id);
    }

    fn _read_dir_entry(&self,entry_id:usize)->vfs::Result<DiskEntry>{

        let mut buf:DiskEntry=unsafe{uninit_memory()};
        // have specific error
        self.read_at(entry_id*DIRENT_SIZE,buf.as_buf_mut())?;
        Ok(buf)
    }
    fn _write_dir_entry(&self,entry_id:usize,disk_entry:& DiskEntry)->vfs::Result<()>{
        self.write_at(entry_id*DIRENT_SIZE,disk_entry.as_buf())?;
        Ok(())
    }
    fn _append_dir_entry(&self,disk_entry:& DiskEntry){

    }
    fn _remove_dir_entry(&self,id:usize){

    }
}