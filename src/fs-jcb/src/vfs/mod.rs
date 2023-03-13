

use core::any::Any;
use core::str;


use crate::block_device::BlockDevice;
use alloc::{string::String, sync::Arc, vec::Vec};
use core::result;

pub trait Inode: Any + Sync + Send{

    fn metadata(&self)->Result<MetaData>{
        Err(FsError::NotSupported)
    }
    fn set_metadata(&self)->Result<()>{
        Err(FsError::NotSupported)
    }

    /// create new node under the current node
    fn create(&self,name:&str,type_:FileType,mode: u32)->Option<Arc<dyn Inode>>;
    /// find node in the directory by name
    fn find(&self,name:&str)->Result<Arc<dyn Inode>>;

    fn get_entry_with_meta_data(&self,entry_id:usize)-> Result<(MetaData, String)>;

    fn get_entry(&self,entry_id:usize)-> Result<String>;

    fn read_at(&self,offset:usize,buf:&[u8])->Result<usize>{Err(FsError::NotSupported)}

    fn write_at(&self,offset:usize,buf:&mut [u8])->Result<usize>{Err(FsError::NotSupported)}

    fn resize(&self)->Result<()>{
        Err(FsError::NotSupported)
    }
    fn fs(&self)->Arc<dyn FileSystem>;
}
impl dyn Inode{
    fn list(&self)->Result<Vec<String>>{

        let info=self.metadata()?;
        if info.type_!=FileType::Dir{
            return Err(FsError::NotDir)
        }
        (0..).map(|id| self.get_entry(id))
            .take_while(|result| result.is_ok())
            .filter_map(|result| result.ok())
            .collect()
    }

    fn find_by_path(&self,path:&str)->Result<Arc<dyn Inode>>{
        self.find_by_path_follow(path,0)
    }

    fn find_by_path_follow(&self,path:&str,follow_times:usize)->Result<Arc<dyn Inode>> {
        if self.metadata()?.type_ != FileType::Dir {
            return Err(FsError::NotDir);
        }

        let (mut result, mut rest_path) = {
            if let Some(rest) = path.strip_prefix('/') {
                (self.fs().root_inode(),String::from(rest) )
            } else {
                /// TODO FIXME
                (self.find(".")?,String::from(path))
            }
        };
        while !rest_path.is_empty() {
            if result.metadata()?.type_==FileType::File {
                return Err(FsError::NotDir);
            }
            let name={
                if let Some(pos) = rest_path.find('/'){
                    let clip=String::from(&rest_path[0..pos]);
                    rest_path= String::from(&rest_path[pos + 1..]);
                    clip
                }else{
                    rest_path.clone()
                }
            };
            let inode=result.find(&name)?;
            /// implement the function about symlinks
            if inode.metadata()?.type_==FileType::SymLink&&follow_times>0{
                let mut buf = [0u8;256];

                let len=result.read_at(0,&buf)?;
                let link_path=String::from(str::from_utf8(&buf[0..len]).map_err(|e| FsError::NotDir)?);

                let new_path=link_path+"/"+&rest_path;

                return self.find_by_path_follow(&new_path,follow_times-1);
            }else{
                result=inode;
            }
        }
        Ok(result)
    }
}

pub trait FileSystem:Sync+Send{

    fn root_inode(&self)->Arc<dyn Inode>;
}

impl dyn FileSystem{

}


pub struct MetaData{
    pub size: usize,
    /// A file system-specific preferred I/O block size for this object.
    /// In some file system types, this may vary from file to file.
    pub blk_size: usize,
    /// Size in blocks
    pub blocks: usize,
    /// Time of last access
    pub atime: Timespec,
    /// Time of last modification
    pub mtime: Timespec,
    /// Time of last change
    pub ctime: Timespec,

    pub dev: usize,

    pub inode_id :usize,

    pub type_: FileType,

    pub permission:u16,

    /// User ID
    pub uid: usize,
    /// Group ID
    pub gid: usize,
    /// Raw device id
    /// e.g. /dev/null: makedev(0x1, 0x3)
    pub rdev: usize, // (major << 8) | minor
}

// Note: IOError/NoMemory always lead to a panic since it's hard to recover from it.
//       We also panic when we can not parse the fs on disk normally
#[derive(Debug, Eq, PartialEq)]
pub enum FsError {
    NotSupported,  // E_UNIMP, or E_INVAL
    NotFile,       // E_ISDIR
    IsDir,         // E_ISDIR, used only in link
    NotDir,        // E_NOTDIR
    EntryNotFound, // E_NOENT
    EntryExist,    // E_EXIST
    NotSameFs,     // E_XDEV
    InvalidParam,  // E_INVAL
    NoDeviceSpace, // E_NOSPC, but is defined and not used in the original ucore, which uses E_NO_MEM
    DirRemoved,    // E_NOENT, when the current dir was remove by a previous unlink
    DirNotEmpty,   // E_NOTEMPTY
    WrongFs,       // E_INVAL, when we find the content on disk is wrong when opening the device
    DeviceError,
    IOCTLError,
    NoDevice,
    Again,       // E_AGAIN, when no data is available, never happens in fs
    SymLoop,     // E_LOOP
    Busy,        // E_BUSY
    Interrupted, // E_INTR
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Timespec {
    pub sec: i64,
    pub nsec: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileType {
    File,
    Dir,
    SymLink,
    CharDevice,
    BlockDevice,
    NamedPipe,
    Socket,
}

pub type Result<T>=result::Result<T,FsError>;

