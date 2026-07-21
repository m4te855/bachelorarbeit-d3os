// The structure is copied from unsupported.rs
#![allow(unused_imports)]
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::arch::asm;

use syscall::*; // syscall from D3OS
use alloc::string::{String, ToString};

pub struct Stdin;
pub struct Stdout;
pub type Stderr = Stdout;

//////////////////////////////////////////////////////////////////////

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() { return Ok(0); }
        // 0 == TerminalMode::Canonical (blocks until Enter, echoes)
        let res = syscall(SystemCall::TerminalReadInput,
            &[buf.as_mut_ptr() as usize, buf.len(), 0]);
        match res {
            Ok(n) => Ok(n),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "read failed")),
        }
    }
    /*
    #[inline]
    fn read_buf(&mut self, _cursor: BorrowedCursor<'_>) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn read_vectored(&mut self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)
    }

    #[inline]
    fn is_read_vectored(&self) -> bool {
        // Do not force `Chain<Empty, T>` or `Chain<T, Empty>` to use vectored
        // reads, unless the other reader is vectored.
        false
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if !buf.is_empty() { Err(io::Error::READ_EXACT_EOF) } else { Ok(()) }
    }

    #[inline]
    fn read_buf_exact(&mut self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        if cursor.capacity() != 0 { Err(io::Error::READ_EXACT_EOF) } else { Ok(()) }
    }

    #[inline]
    fn read_to_end(&mut self, _buf: &mut Vec<u8>) -> io::Result<usize> {
        Ok(0)
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        Ok(0)
    }*/
}

//////////////////////////////////////////////////////////////////////

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // D3OS syscall:
        let res = syscall(
            SystemCall::TerminalWriteOutput,
            &[buf.as_ptr() as usize, buf.len()],
        );
            
        match res {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "write failed")),
        }
    }

    /*#[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let total_len = bufs.iter().map(|b| b.len()).sum();
        Ok(total_len)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn write_all(&mut self, _buf: &[u8]) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all_vectored(&mut self, _bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        Ok(())
    }

    // Keep the default write_fmt so the `fmt::Arguments` are still evaluated.*/

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////

pub const STDIN_BUF_SIZE: usize = 128;

pub fn is_ebadf(_err: &io::Error) -> bool {
    true
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr::new())
}
