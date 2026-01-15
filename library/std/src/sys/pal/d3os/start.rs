use crate::ffi::{c_char, c_int};
use crate::ptr;
use syscall::{SystemCall, syscall};
use crate::io;
use crate::io::Write;

unsafe extern "C" {
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
#[allow(unused)]
pub extern "C" fn _start() {
    
    unsafe {
        main(0, ptr::null());
    };
    
    let mut buffer = io::stdout();
    let x = buffer.write(&"12345".as_bytes());

    let s = "Start ist erfolgreich durchgelaufen!\n";
    let buf: &[u8] = s.as_bytes();

    let _res = syscall(
        SystemCall::TerminalWriteOutput,
        &[buf.as_ptr() as usize, buf.len()],
    );

    syscall(SystemCall::ProcessExit, &[]).expect("Failed to exit process");
}