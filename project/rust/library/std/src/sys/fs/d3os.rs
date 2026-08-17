// The fd-based structure is modelled on solid.rs; the un-implemented surface
// is reused from unsupported.rs (same trick as vexos.rs).
// Implemented against the D3OS naming service: open, read, write, seek, close.

// Support for mkdir and readdir is left as future work. Most of the remaining stubs
// cannot be implemented at all yet: D3OS currently has no syscall for unlink, rename, rmdir,
// stat, chmod, utimes, link or symlink.

use crate::ffi::CString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::Path;
use crate::sys::{unsupported, unsupported_err};

use syscall::*; // syscall from D3OS

// Reuse the whole un-implemented surface (types + free functions) from
// unsupported.rs; we only implement File and OpenOptions ourselves.
#[allow(dead_code)]
#[path = "unsupported.rs"]
mod unsupported_fs;
pub use unsupported_fs::{
    DirBuilder, DirEntry, FileAttr, FilePermissions, FileType, FileTimes, ReadDir, canonicalize,
    copy, exists, link, lstat, readdir, readlink, remove_dir_all, rename, rmdir, set_perm,
    set_times, set_times_nofollow, stat, symlink, unlink,
};

// D3OS open flags, mirroring the OpenOptions bitflags in naming/shared_types.rs.
// Now real powers of two so OR-combining them is safe.
const D3OS_READONLY: usize = 1;
const D3OS_READWRITE: usize = 2;
const D3OS_CREATE: usize = 4;
const D3OS_EXCLUSIVE: usize = 8;

// Seek origins (SeekOrigin enum also in naming/shared_types.rs):
const D3OS_SEEK_START:   usize = 1;
const D3OS_SEEK_END:     usize = 2;
const D3OS_SEEK_CURRENT: usize = 3;

// Unlike solid (which niches a -1 error sentinel), D3OS returns
// Result<usize, Errno>, so a plain usize is sufficient.
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
// Everything in the following impl is 1:1 from solid.rs except to_flags; solid's
// system-specific custom_flags field was dropped, since D3OS has no OpenOptionsExt.
// `truncate` and the append semantics have no D3OS equivalent yet (`append` only
// counts as write access).
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
    // solid.rs splits this into get_access_mode and get_creation_mode, mapping the two
    // POSIX flag groups and rejecting invalid flag combinations on the way. That validation
    // is dropped here (an empty OpenOptions silently becomes READONLY), and a single mapping
    // method is enough; what D3OS does check happens in the kernel instead (EINVAL for
    // DIRECTORY|CREATE, EEXIST for CREATE|EXCLUSIVE).
    // Access bits are passed for forward compatibility even though the kernel/tmpfs
    // does not enforce them yet (`_options` in read/write (tmpfs), and similar for open_objects).
    fn to_flags(&self) -> usize {
        let mut flags = if self.write || self.append { D3OS_READWRITE }
            else { D3OS_READONLY };
        if self.create || self.create_new { flags |= D3OS_CREATE; }
        // create_new means "must not exist yet", which is what EXCLUSIVE adds.
        if self.create_new { flags |= D3OS_EXCLUSIVE; }
        flags
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        // D3OS expects a null-terminated utf-8 path; the kernel does from_utf8 on it.
        let path =
            path.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput,
                "path is not valid utf-8"))?;
        let c_path = CString::new(path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a null byte"))?;
        
        // Arguments match naming/src/lib.rs (path pointer, flags).
        match syscall(SystemCall::Open, &[c_path.as_ptr() as usize, opts.to_flags()]) {
            Ok(file_descriptor) => Ok(File(file_descriptor)),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "open failed")),
        }
    }

    // read and write mirror naming/src/lib.rs as well: (handle, buffer, length).
    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        // D3OS rejects zero-length reads with EINVAL, but io::Read requires Ok(0).
        if buffer.is_empty() { return Ok(0); }
        match syscall(SystemCall::Read, &[self.0, buffer.as_mut_ptr() as usize, buffer.len()]) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "read failed")),
        }
    }

    pub fn write(&self, buffer: &[u8]) -> io::Result<usize> {
        // Same as in read: a zero-length write is not an error, it writes nothing.
        if buffer.is_empty() { return Ok(0); }
        match syscall(SystemCall::Write, &[self.0, buffer.as_ptr() as usize, buffer.len()]) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "write failed")),
        }
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        // D3OS SeekOrigin: Start=1, End=2, Current=3; the offset is a usize,
        // so negative offsets (End/Current) cannot be expressed yet. The kernel adds the offset.
        let (origin, offset) = match pos {
            SeekFrom::Start(offset)   => (D3OS_SEEK_START,   offset as usize),
            SeekFrom::End(offset)     => (D3OS_SEEK_END,     non_negative_offset(offset)?),
            SeekFrom::Current(offset) => (D3OS_SEEK_CURRENT, non_negative_offset(offset)?),
        };
        // Unlike SOLID_FS_Lseek, which forces solid.rs to call tell() afterwards,
        // the D3OS syscall returns the new position directly.
        match syscall(SystemCall::Seek, &[self.0, offset, origin]) {
            Ok(new_position) => Ok(new_position as u64),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "seek failed")),
        }
    }

    // Mirroring sys/fs/motor.rs: returns the current stream position via a zero-offset seek.
    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    // Writes go straight to the kernel, so there is nothing buffered on the std
    // side; flush and the two sync variants have nothing left to do.
    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn fsync(&self) -> io::Result<()> {
        self.flush()
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.flush()
    }

    // D3OS has no scatter/gather syscalls, so is_read_vectored / is_write_vectored
    // report false and the default helpers just forward the first non-empty buffer
    // to the single-buffer path (read_buf as in motor.rs, the vectored pair as in solid.rs).
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

    // Optimization hint for std. Returns None since file_attr is unsupported.
    pub fn size(&self) -> Option<io::Result<u64>> {
        None
    }

    // Not supported by the D3OS naming service.
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

// Seek syscall takes usize; reject negative offsets early to prevent wrap-around.
fn non_negative_offset(offset: i64) -> io::Result<usize> {
    if offset < 0 {
        Err(io::Error::new(io::ErrorKind::InvalidInput,
            "D3OS seek does not support negative offsets"))
    } else {
        Ok(offset as usize)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // Close the handle; nothing sensible to do with an error while dropping.
        let _ = syscall(SystemCall::Close, &[self.0]);
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mirrors solid.rs: using `&self.0` directly since D3OS holds a plain `usize`.
        f.debug_struct("File").field("fd", &self.0).finish()
    }
}