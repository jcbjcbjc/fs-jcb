use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use fs_jcb::FileType;
use crate::util::Dirty;
use crate::vfs::{FileType, Timespec};
use alloc::str;

#[derive(Debug,Clone)]
pub struct DiskEntry{
    pub inode_id:InodeId,
    pub name:Str256
}

impl<'a> From<&'a str> for Str256{
    fn from(s :&'a str)->Self{
        let mut ret = [0u8;256];
        ret[0..s.len()].copy_from_slice(s.as_ref());
        Str256(ret)
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct Str256(pub [u8; 256]);

#[repr(C)]
#[derive(Clone)]
pub struct Str32(pub [u8; 32]);

impl AsRef<str> for Str256 {
    fn as_ref(&self) -> &str {
        let len = self.0.iter().enumerate().find(|(_, &b)| b == 0).unwrap().0;
        str::from_utf8(&self.0[0..len]).unwrap()
    }
}

impl AsRef<str> for Str32 {
    fn as_ref(&self) -> &str {
        let len = self.0.iter().enumerate().find(|(_, &b)| b == 0).unwrap().0;
        str::from_utf8(&self.0[0..len]).unwrap()
    }
}

impl Debug for Str256 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.as_ref())
    }
}

impl Debug for Str32 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.as_ref())
    }
}

/// On-disk superblock
#[repr(C)]
#[derive(Debug)]
pub struct SuperBlock {
    /// magic number, should be SFS_MAGIC
    pub magic: u32,
    /// number of blocks in fs
    pub blocks: u32,
    /// number of unused blocks in fs
    pub unused_blocks: u32,
    /// information for sfs
    pub info: Str32,
    /// number of freemap blocks
    pub free_map_blocks: u32,
}
/// inode (on disk)
#[repr(C)]
#[derive(Debug)]
pub struct DiskINode {
    /// size of the file (in bytes)
    /// undefined in dir (256 * #entries ?)
    pub size: u32,
    /// one of SYS_TYPE_* above
    pub type_: FileType,
    /// number of hard links to this file
    /// Note: "." and ".." is counted in this nlinks
    pub nlinks: u16,
    /// number of blocks
    pub blocks: u32,
    /// direct blocks
    pub direct: [u32; NDIRECT],
    /// indirect blocks
    pub indirect: u32,
    /// double indirect blocks
    pub db_indirect: u32,
    /// device inode id for char/block device (major, minor)
    pub device_inode_id: usize,
    /// Time of last access
    pub atime: Timespec,
    /// Time of last modification
    pub mtime: Timespec,
    /// Time of last change
    pub ctime: Timespec,
}
impl DiskINode {
    pub const fn new_file() -> Self {
        DiskINode {
            size: 0,
            type_: FileType::File,
            nlinks: 0,
            blocks: 0,
            direct: [0; NDIRECT],
            indirect: 0,
            db_indirect: 0,
            device_inode_id: NODEVICE,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
        }
    }
    pub const fn new_symlink() -> Self {
        DiskINode {
            size: 0,
            type_: FileType::SymLink,
            nlinks: 0,
            blocks: 0,
            direct: [0; NDIRECT],
            indirect: 0,
            db_indirect: 0,
            device_inode_id: NODEVICE,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
        }
    }
    pub const fn new_dir() -> Self {
        DiskINode {
            size: 0,
            type_: FileType::Dir,
            nlinks: 0,
            blocks: 0,
            direct: [0; NDIRECT],
            indirect: 0,
            db_indirect: 0,
            device_inode_id: NODEVICE,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
        }
    }
    pub const fn new_chardevice(device_inode_id: usize) -> Self {
        DiskINode {
            size: 0,
            type_: FileType::CharDevice,
            nlinks: 0,
            blocks: 0,
            direct: [0; NDIRECT],
            indirect: 0,
            db_indirect: 0,
            device_inode_id,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
        }
    }
}
pub type BlockId=usize;
pub type InodeId=usize;
pub type FreeMap=Dirty<BitVec<Lsb0, u8>>;

pub trait Alloc{
    fn alloc(&self)->Option<usize>;
    fn dealloc(&self,id:usize)->vfs::Result<()>;
}
impl Alloc for FreeMap{
    fn alloc(&self) -> Option<usize> {
        let id=(0..self.len()).find(|&i| self[i]);
        if let Some(alloc_id)=id{
            self[alloc_id]=false;
        }
        id
    }
    fn dealloc(&self,id:usize) -> vfs::Result<()> {
        if id>self.len(){
            Err()
        }
        self[id]=true;
        Ok(())
    }
}

pub trait AsBuf{
    fn as_buf(&self)->&[u8]{

    }
    fn as_buf_mut(&self)->&mut [u8]{

    }
}
impl AsBuf for DiskEntry{}


pub const NODEVICE: usize = 100;

/// magic number for sfs
pub const MAGIC: u32 = 0x2f8dbe2b;
/// size of block
pub const BLKSIZE: usize = 1usize << BLKSIZE_LOG2;
/// log2( size of block )
pub const BLKSIZE_LOG2: u8 = 12;
/// number of direct blocks in inode
pub const NDIRECT: usize = 12;
/// default sfs infomation string
pub const DEFAULT_INFO: &str = "simple file system";
/// max length of infomation
pub const MAX_INFO_LEN: usize = 31;
/// max length of filename
pub const MAX_FNAME_LEN: usize = 255;
/// max file size in theory (48KB + 4MB + 4GB)
/// however, the file size is stored in u32
pub const MAX_FILE_SIZE: usize = 0xffffffff;
/// block the superblock lives in
pub const BLKN_SUPER: BlockId = 0;
/// location of the root dir inode
pub const BLKN_ROOT: BlockId = 1;
/// 1st block of the freemap
pub const BLKN_FREEMAP: BlockId = 2;
/// number of bits in a block
pub const BLKBITS: usize = BLKSIZE * 8;
/// size of one entry
pub const ENTRY_SIZE: usize = 4;
/// number of entries in a block
pub const BLK_NENTRY: usize = BLKSIZE / ENTRY_SIZE;
/// size of a dirent used in the size field
pub const DIRENT_SIZE: usize = MAX_FNAME_LEN + 1 + ENTRY_SIZE;
/// max number of blocks with direct blocks
pub const MAX_NBLOCK_DIRECT: usize = NDIRECT;
/// max number of blocks with indirect blocks
pub const MAX_NBLOCK_INDIRECT: usize = NDIRECT + BLK_NENTRY;
/// max number of blocks with double indirect blocks
pub const MAX_NBLOCK_DOUBLE_INDIRECT: usize = NDIRECT + BLK_NENTRY + BLK_NENTRY * BLK_NENTRY;

