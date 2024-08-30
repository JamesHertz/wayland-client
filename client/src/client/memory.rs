use memmap::{MmapMut, MmapOptions};
use std::{
    fs::File,
    io::Read,
    iter,
    os::fd::{AsRawFd, FromRawFd},
    ops::{Deref, DerefMut}
};

use crate::{
    Result, error::fallback_error
};

pub struct SharedBuffer {
    pub shm_file: File,
    pub data: MmapMut,
}

impl SharedBuffer {
    pub fn as_file_descriptor(&self) -> i32 {
        self.shm_file.as_raw_fd()
    }

    pub fn alloc(size: usize) -> Result<Self> {
        let filename = b"my-own-custom-file\0".as_ptr() as *const libc::c_char;
        let fd = unsafe {
            libc::shm_open(
                filename,
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o666,
            )
        };

        if fd < 0 {
            return Err(fallback_error!(
                "Error creating with shm_open '{}'",
                errno::errno()
            ));
        }

        let res = unsafe { libc::shm_unlink(filename) };
        if res < 0 {
            return Err(fallback_error!("Error unlinking '{}'", errno::errno()));
        }

        let shm_file = unsafe { File::from_raw_fd(fd) };

        shm_file.set_len(size as u64)?;

        let data = unsafe { MmapOptions::new().map_mut(&shm_file)? };

        Ok(Self { shm_file, data })
    }
}


impl Deref for  SharedBuffer {
    type Target = MmapMut;

    fn deref(&self) -> &Self::Target {
        &self.data
    }

}

impl DerefMut for SharedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }

}
