//! A naive LRU cache layer for `BlockDevice`
use super::*;
use alloc::{collections, vec, vec::Vec};
use alloc::collections::BTreeMap;
use spin::{Mutex, MutexGuard};

struct Buf{
    content:Vec<u8>,
    buf_status:BufStatus,
}
enum BufStatus{
    Unused,
    Valid(BlockId),
    Dirty(BlockId)
}
struct BufAllocator{
    current_id:usize,
}
impl BufAllocator{
    fn alloc(&mut self, capacity:usize) ->Option<usize>{
        if self.current_id<capacity{
            self.current_id=self.current_id+1;
            return Some(self.current_id-1);
        }
        return None;
    }
}

pub struct BlockCache<T:BlockDevice>{
    capacity:usize,
    device:T,
    bufs:Vec<Mutex<Buf>>,
    lru:Mutex<LRU>,
    map_list:Mutex<collections::BTreeMap<BlockId,usize>>,
    buf_allocator:Mutex<BufAllocator>
}

impl<T:BlockDevice> BlockCache<T>{
    pub fn new(dev:T,size:usize)->Self{
        Self{
            capacity: size,
            device: dev,
            bufs: vec![Mutex::new(Buf{
                content: vec!(0;1<<T::BLOCK_SIZE_LOG2 as usize),
                buf_status: BufStatus::Unused
            }); size],
            lru: Mutex::new(LRU::new(size)),
            map_list: Mutex::new(BTreeMap::new()),
            buf_allocator: Mutex::new(BufAllocator{ current_id: 0})
        }
    }

    fn get_buf(&self,block_id:BlockId)->MutexGuard<Buf>{
        // get buf
        let (i,buf)=self._get_buf(block_id);
        // update lru
        self.lru.lock().visit(i);
        // update map
        self.map_list.lock().insert(block_id,i);

        buf
    }
    fn _get_buf(&self,block_id:BlockId)->(usize,MutexGuard<Buf>){
        if let Some(&id)=self.map_list.lock().get(&block_id){
            return (id,self.bufs[id].lock())
        }
        self._get_unused()
    }
    fn _get_unused(&self)->(usize,MutexGuard<Buf>){
        if let Some(id)=self.buf_allocator.lock().alloc(self.capacity){
            (id,self.bufs[id].lock())
        }

        let id=self.lru.lock().tail();
        let mut remove_buf =self.bufs[id].lock();

        //write back into the disk
        self.write_back(&mut remove_buf).expect("write back fail");

        // remove from map_list
        match remove_buf.buf_status{
            BufStatus::Valid(block_id) => self.map_list.lock().remove(&block_id),
            BufStatus::Dirty(block_id) => self.map_list.lock().remove(&block_id),
            _ =>{}
        }

        // change into unused
        remove_buf.buf_status=BufStatus::Unused;

        (id,remove_buf)
    }
    fn write_back(&self,buf:&mut Buf)->Result<()>{
        if let Some(BufStatus::Dirty(block_id))=buf.buf_status {
            self.device.write_at(block_id,&mut buf.content)?;
            buf.buf_status=BufStatus::Valid(block_id);
        }
        Ok(())
    }
}


///detail information in the connection between device and cpu
///for a device,if cpu wanna to read from or write to the device
///read:
/// cpu will send a io signal to the DMA module ,and make the process exit the cpu
/// ,DMA module will transfer the data form disk to the cache in the kernel witch just called page cache like the follows,
/// and DMA will cause a interrupt to awake the process ,and the process copy the kernel cache to the user space cache
/// write:
/// just like read,cpu will first copy user space cache to the kernel cache(page cache just like the follows),and then symbolise a dirty label,
/// in the write_back process,cpu will send a io signal to the DMA module and make self exit the cpu ........
///
///

impl<T:BlockDevice> BlockDevice for BlockCache<T>{
    const BLOCK_SIZE_LOG2: u8 = T::BLOCK_SIZE_LOG2;

    fn read_at(&self, block_id: BlockId, dst_buf: &mut [u8]) -> Result<()> {
        let mut buf=self.get_buf(block_id);
        if let Some(BufStatus::Unused)=buf.buf_status{
            buf.buf_status=BufStatus::Valid(block_id);
            self.device.read_at(block_id, &mut buf.content)?;
        }

        let len=1<<T::BLOCK_SIZE_LOG2 as usize;
        dst_buf[..len].copy_from_slice(buf.content.as_slice());

        Ok(())
    }

    fn write_at(&self, block_id: BlockId, from_buf: &[u8]) -> Result<()> {
        let mut buf=self.get_buf(block_id);

        buf.buf_status=BufStatus::Dirty(block_id);

        let len=1<<T::BLOCK_SIZE_LOG2 as usize;
        buf.content.copy_from_slice(&from_buf[..len]);
        Ok(())
    }

    fn sync(&self) -> Result<()> {
        for buf in self.bufs.iter(){
            self.write_back(&mut buf.lock()).expect("write back fail");
        }
        Ok(())
    }
}


struct LRU{
    list:collections::LinkedList<usize>,
}

impl LRU{
    pub fn new(size:usize)->Self{
        let arr:Vec<usize>=(0..size).collect();
        let mut lru=LRU{
            list:collections::LinkedList::new()
        };
        for i in 0..size{
            lru.list.push_back(i);
        }
        lru
    }
    pub fn tail(&self)->usize{
        return *self.list.back().unwrap();
    }
    pub fn visit(&mut self, id:usize){
        //remove the inode
        let (inode,_)=self.list.iter().enumerate().find(|(_,&num)| num==id).unwrap();
        let remove=self.list.remove(inode);
        //push to the head
        self.list.push_front(remove);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spin::Mutex;

    impl BlockDevice for Mutex<[u8; 16]> {
        const BLOCK_SIZE_LOG2: u8 = 2;
        fn read_at(&self, block_id: BlockId, buf: &mut [u8]) -> Result<()> {
            if block_id >= 4 {
                return Err(DevError);
            }
            let begin = block_id << 2;
            buf[..4].copy_from_slice(&self.lock().unwrap()[begin..begin + 4]);
            Ok(())
        }
        fn write_at(&self, block_id: BlockId, buf: &[u8]) -> Result<()> {
            if block_id >= 4 {
                return Err(DevError);
            }
            let begin = block_id << 2;
            self.lock().unwrap()[begin..begin + 4].copy_from_slice(&buf[..4]);
            Ok(())
        }
        fn sync(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn read() {
        let buf: Mutex<[u8; 16]> =
            Mutex::new([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let mut res: [u8; 6] = [0; 6];

        // all inside
        let ret = Device::read_at(&buf, 3, &mut res);
        assert_eq!(ret, Ok(6));
        assert_eq!(res, [3, 4, 5, 6, 7, 8]);

        // partly inside
        let ret = Device::read_at(&buf, 11, &mut res);
        assert_eq!(ret, Ok(5));
        assert_eq!(res, [11, 12, 13, 14, 15, 8]);

        // all outside
        let ret = Device::read_at(&buf, 16, &mut res);
        assert_eq!(ret, Ok(0));
        assert_eq!(res, [11, 12, 13, 14, 15, 8]);
    }

    #[test]
    fn write() {
        let buf: Mutex<[u8; 16]> = Mutex::new([0; 16]);
        let res: [u8; 6] = [3, 4, 5, 6, 7, 8];

        // all inside
        let ret = Device::write_at(&buf, 3, &res);
        assert_eq!(ret, Ok(6));
        assert_eq!(
            *buf.lock().unwrap(),
            [0, 0, 0, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0]
        );

        // partly inside
        let ret = Device::write_at(&buf, 11, &res);
        assert_eq!(ret, Ok(5));
        assert_eq!(
            *buf.lock().unwrap(),
            [0, 0, 0, 3, 4, 5, 6, 7, 8, 0, 0, 3, 4, 5, 6, 7]
        );

        // all outside
        let ret = Device::write_at(&buf, 16, &res);
        assert_eq!(ret, Ok(0));
        assert_eq!(
            *buf.lock().unwrap(),
            [0, 0, 0, 3, 4, 5, 6, 7, 8, 0, 0, 3, 4, 5, 6, 7]
        );
    }
}



