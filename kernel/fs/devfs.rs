use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, INode, INodeNo},
    stat::{FileMode, Stat, S_IFDIR},
};
use crate::{
    arch::print_str,
    process::WaitQueue,
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
use crossbeam::queue::ArrayQueue;
use penguin_utils::once::Once;

static ROOT_DIR: Once<Arc<dyn Directory>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
pub static CONSOLE_FILE: Once<Arc<ConsoleFile>> = Once::new();

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();

pub struct DevFs {}

impl DevFs {
    pub fn new() -> DevFs {
        ROOT_DIR.init(|| Arc::new(DevRootDir::new()) as Arc<dyn Directory>);
        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        CONSOLE_FILE.init(|| Arc::new(ConsoleFile::new()));
        DevFs {}
    }
}

impl FileSystem for DevFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(ROOT_DIR.clone())
    }
}

/// The `/dev` directory.
struct DevRootDir {}
impl DevRootDir {
    pub fn new() -> DevRootDir {
        DevRootDir {}
    }
}

impl Directory for DevRootDir {
    fn lookup(&self, name: &str) -> Result<DirEntry> {
        match name {
            "null" => Ok(DirEntry {
                inode: INode::FileLike(NULL_FILE.clone()),
            }),
            "console" => Ok(DirEntry {
                inode: INode::FileLike(CONSOLE_FILE.clone()),
            }),
            _ => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(1),
            mode: FileMode::new(S_IFDIR | 0o755),
            ..Stat::zeroed()
        })
    }
}

/// The `/dev/null` file.
struct NullFile {}
impl NullFile {
    pub fn new() -> NullFile {
        NullFile {}
    }
}

impl FileLike for NullFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
}

/// The `/dev/console` file.
pub struct ConsoleFile {
    input: ArrayQueue<u8>,
    wait_queue: WaitQueue,
}

impl ConsoleFile {
    pub fn new() -> ConsoleFile {
        ConsoleFile {
            wait_queue: WaitQueue::new(),
            input: ArrayQueue::new(64),
        }
    }

    pub fn input_char(&self, ch: char) {
        self.write(0, &[ch as u8]).ok();
        self.input.push(ch as u8).ok();
        self.wait_queue.wake_one();
    }
}

impl FileLike for ConsoleFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        loop {
            let mut read_len = 0;
            while let Some(ch) = self.input.pop() {
                buf[read_len] = ch as u8;
                read_len += 1;
            }

            if read_len > 0 {
                return Ok(read_len);
            }

            self.wait_queue.sleep();
        }
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        print_str(b"\x1b[1m");
        print_str(buf);
        print_str(b"\x1b[0m");
        Ok(buf.len())
    }
}

pub fn init() {
    DEV_FS.init(|| Arc::new(DevFs::new()));
}
