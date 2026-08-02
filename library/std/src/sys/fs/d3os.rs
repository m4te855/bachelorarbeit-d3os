// The fd-based structure is modelled on solid.rs; the un-implemented surface
// is reused from unsupported.rs (same trick as vexos.rs).

use crate::ffi::CString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::Path;
use crate::sys::{unsupported, unsupported_err};

use syscall::*; // syscall from D3OS, same crate stdio/d3os.rs uses

// reuse the whole un-implemented surface (types + free functions) from
// unsupported.rs; we only implement File and OpenOptions ourselves.
#[allow(dead_code)]
#[path = "unsupported.rs"]
mod unsupported_fs;
pub use unsupported_fs::{
    DirBuilder, DirEntry, FileAttr, FilePermissions, FileType, FileTimes, ReadDir, canonicalize,
    copy, exists, link, lstat, readdir, readlink, remove_dir_all, rename, rmdir, set_perm,
    set_times, set_times_nofollow, stat, symlink, unlink,
};

// d3os open flags (naming/shared_types.rs) — now real powers of two
// so OR-combining them is safe.
const D3OS_READONLY: usize = 1;
const D3OS_READWRITE: usize = 2;
const D3OS_CREATE: usize = 4;
const D3OS_EXCLUSIVE: usize = 8; 

// seek origins (SeekOrigin enum)
const D3OS_SEEK_START:   usize = 1;
const D3OS_SEEK_END:     usize = 2;
const D3OS_SEEK_CURRENT: usize = 3;

pub struct File(usize);

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    // map std's OpenOptions onto d3os flag bits (see the note above the consts).
    // truncate/append have no d3os equivalent yet and are ignored for now.
    fn to_flags(&self) -> usize {
        let mut flags = if self.write || self.append { D3OS_READWRITE } 
            else { D3OS_READONLY };
        if self.create || self.create_new { flags |= D3OS_CREATE; }
        if self.create_new { flags |= D3OS_EXCLUSIVE; }
        flags
}
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        // d3os expects a null-terminated utf-8 path; the kernel does from_utf8 on it.
        let path =
            path.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path is not valid utf-8"))?;
        let c_path = CString::new(path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a null byte"))?;

        match syscall(SystemCall::Open, &[c_path.as_ptr() as usize, opts.to_flags()]) {
            Ok(file_descriptor) => Ok(File(file_descriptor)),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "open failed")),
        }
    }

    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        match syscall(SystemCall::Read, &[self.0, buffer.as_mut_ptr() as usize, buffer.len()]) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "read failed")),
        }
    }

    pub fn write(&self, buffer: &[u8]) -> io::Result<usize> {
        match syscall(SystemCall::Write, &[self.0, buffer.as_ptr() as usize, buffer.len()]) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "write failed")),
        }
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        // d3os SeekOrigin: Start=1, End=2, Current=3; the offset is a usize,
        // so negative offsets (End/Current) cannot be expressed yet.
        let (origin, offset) = match pos {
            SeekFrom::Start(offset)   => (D3OS_SEEK_START,   offset as usize),
            SeekFrom::End(offset)     => (D3OS_SEEK_END,     non_negative_offset(offset)?),
            SeekFrom::Current(offset) => (D3OS_SEEK_CURRENT, non_negative_offset(offset)?),
        };
        match syscall(SystemCall::Seek, &[self.0, offset, origin]) {
            Ok(new_position) => Ok(new_position as u64),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "seek failed")),
        }
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn fsync(&self) -> io::Result<()> {
        self.flush()
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.flush()
    }

    // vectored / buffered variants fall back onto the byte-wise path
    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        crate::io::default_read_vectored(|buffer| self.read(buffer), bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        crate::io::default_read_buf(|buffer| self.read(buffer), cursor)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(|buffer| self.write(buffer), bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        None
    }

    // not supported by the d3os naming service (yet)
    pub fn file_attr(&self) -> io::Result<FileAttr> {
        unsupported()
    }
    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        unsupported()
    }
    pub fn duplicate(&self) -> io::Result<File> {
        unsupported()
    }
    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }
    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        unsupported()
    }
    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }
    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }
    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }
    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }
    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }
}

fn non_negative_offset(offset: i64) -> io::Result<usize> {
    if offset < 0 {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "d3os seek does not support negative offsets"))
    } else {
        Ok(offset as usize)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // close the handle; nothing sensible to do with an error while dropping.
        let _ = syscall(SystemCall::Close, &[self.0]);
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("fd", &self.0).finish()
    }
}