pub mod block_cache;

use crate::{util::*, vfs::Timespec};


//pub mod std_impl;

/// A current time provider
pub trait TimeProvider: Send + Sync {
    fn current_time(&self) -> Timespec;
}

/// Interface for FS to read & write
pub trait Device: Send + Sync {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize>;
    fn sync(&self) -> Result<()>;
}

/// Device which can only R/W in blocks
pub trait BlockDevice: Send + Sync {
    const BLOCK_SIZE_LOG2: u8;
    fn read_at(&self, block_id: BlockId, buf: &mut [u8]) -> Result<()>;
    fn write_at(&self, block_id: BlockId, buf: &[u8]) -> Result<()>;
    fn sync(&self) -> Result<()>;
}


/// The error type for device.
#[derive(Debug, PartialEq, Eq)]
pub struct DevError;

/// A specialized `Result` type for device.
pub type Result<T> = core::result::Result<T, DevError>;

pub type BlockId = usize;

macro_rules! try0 {
    ($len:expr, $res:expr) => {
        if $res.is_err() {
            return Ok($len);
        }
    };
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
