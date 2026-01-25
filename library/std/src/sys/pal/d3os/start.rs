use crate::ffi::{c_char, c_int};
use crate::ptr;

use syscall::{SystemCall, syscall};
use concurrent::*;

//use crate::io;
//use crate::io::Write;

unsafe extern "C" {
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
#[allow(unused)]
pub extern "C" fn _start() {
    //initialise thread environment for thread local storage (TLS)
    thread::init_thread_environment();
    
    // main method call
    unsafe {
        main(0, ptr::null());
    };
    
    // write directly via syscall (for debugging and testing, can be removed anytime)
    let s = "Start finished successfully!\n";
    let buf: &[u8] = s.as_bytes();
    let _res = syscall(
        SystemCall::TerminalWriteOutput,
        &[buf.as_ptr() as usize, buf.len()],
    );

    // exit process
    syscall(SystemCall::ProcessExit, &[]).expect("Failed to exit process");
}