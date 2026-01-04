use crate::arch::asm;
use crate::io::{self, Write};

/// stdout writer
pub struct Stdout;
#[allow(dead_code)]
impl Stdout {
    /// Erzeugt einen neuen Stdout-Handle
    pub const fn new() -> Self {
        Stdout
    }
}
#[allow(dead_code)]
impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            unsafe {
                asm!(
                    "int 0x80",
                    in("rax") 0,       // Syscall Nummer (Beispiel)
                    in("rcx") byte as u64
                );
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// stderr writer
pub struct Stderr;
#[allow(dead_code)]
impl Stderr {
    pub const fn new() -> Self {
        Stderr
    }
}
#[allow(dead_code)]
impl Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            unsafe {
                asm!(
                    "int 0x80",
                    in("rax") 0,       // Syscall Nummer (Beispiel)
                    in("rcx") byte as u64
                );
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[allow(dead_code)]
/// Helfer für bestimmte Error-Kinds
pub fn is_ebadf(_err: &io::Error) -> bool {
    // TODO: je nach implementiertem ErrorKind prüfen
    true
}
#[allow(dead_code)]
/// Gibt einen Writer für Panik-Ausgaben zurück
pub fn panic_output() -> Option<impl Write> {
    Some(Stderr::new())
}
