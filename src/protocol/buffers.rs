use color_eyre::eyre::eyre;
use memmap::{MmapMut, MmapOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::{fs::File, io::Read, iter};

pub struct ByteBuffer {
    data: Box<[u8]>,
    head: usize,
    tail: usize,
}

impl ByteBuffer {
    pub fn new(size: usize) -> Self {
        ByteBuffer {
            head: 0,
            tail: 0,
            data: iter::repeat(0u8)
                .take(size)
                .collect::<Vec<u8>>()
                .into_boxed_slice(),
        }
    }

    fn cached_bytes(&self) -> usize {
        self.tail - self.head
    }

    fn tail_space(&self) -> usize {
        self.data.len() - self.tail
    }

    pub fn read_bytes(
        &mut self,
        bytes: usize,
        reader: &mut impl Read,
    ) -> std::io::Result<Option<&[u8]>> {
        assert!(bytes < 1 << 11);
        let cached_bytes = self.cached_bytes();
        assert!(cached_bytes == 0 || cached_bytes < 2 << 14);
        if bytes <= cached_bytes {
            let res = &self.data[self.head..self.head + bytes];
            self.head += bytes;
            return Ok(Some(res));
        }

        let left_space = self.tail_space();
        if cached_bytes + left_space < bytes {
            self.data.copy_within(self.head..self.tail, 0);
            self.head = 0;
            self.tail = cached_bytes;
        }

        let size = reader.read(&mut self.data[self.tail..])?;
        self.tail += size;

        if size + cached_bytes < bytes {
            return Ok(None);
        }

        let res = &self.data[self.head..self.head + bytes];
        self.head += bytes;
        Ok(Some(res))
    }
}

pub struct SharedBuffer {
    pub shm_file: File,
    pub data: MmapMut,
}

impl SharedBuffer {

    pub fn file_fd(&self) -> i32 {
        self.shm_file.as_raw_fd()
    }

    pub fn alloc(size: usize) -> color_eyre::Result<Self> {
        let filename = b"my-own-custom-file\0".as_ptr() as *const libc::c_char;
        let fd = unsafe {
            libc::shm_open(
                filename,
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o666,
            )
        };

        if fd < 0 {
            return Err(eyre!(
                "Error creating with shm_open '{}'",
                errno::errno()
            ));
        }

        let res = unsafe { libc::shm_unlink(filename) };
        if res < 0 {
            return Err(eyre!("Error unlinking '{}'", errno::errno()));
        }

        let shm_file = unsafe { File::from_raw_fd(fd) };

        shm_file.set_len(size as u64)?;

        let data= unsafe { MmapOptions::new().map_mut(&shm_file)? };

        Ok(Self {
            shm_file,
            data,
        })
    }
}
