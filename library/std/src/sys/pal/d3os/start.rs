use crate::ffi::{c_char, c_int};
use crate::ptr;
//use syscall::{SystemCall, syscall};

unsafe extern "C" {
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
#[allow(unused)]
pub extern "C" fn _start() {
    unsafe {
        main(0, ptr::null());
    };

    //syscall(SystemCall::ProcessExit, &[]).expect("Failed to exit process");
    
    loop{};
}