use memmap::{MmapMut, MmapOptions};
use std::{
    ffi::CString, fs::File, ops::{Deref, DerefMut}, os::fd::FromRawFd
};

use crate::{
    Result, error::fallback_error
};

pub struct SharedBuffer (pub MmapMut);

impl SharedBuffer {

    pub fn alloc(size: usize) -> Result<(Self, File)> {
        let filename = CString::new("my-own-custom-file")?;
        let fd = unsafe {
            libc::shm_open(
                filename.as_ptr(),
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

        let res = unsafe { libc::shm_unlink(filename.as_ptr()) };
        if res < 0 {
            return Err(fallback_error!("Error unlinking '{}'", errno::errno()));
        }

        let shm_file = unsafe { File::from_raw_fd(fd) };

        shm_file.set_len(size as u64)?;

        let data = unsafe { MmapOptions::new().map_mut(&shm_file)? };

        Ok((Self(data), shm_file))
    }

}


impl Deref for  SharedBuffer {
    type Target = MmapMut;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
