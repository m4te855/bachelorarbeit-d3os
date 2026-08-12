// thread backend for d3os. spawn goes through the raw ThreadCreate syscall so we
// can hand the boxed ThreadInit to the new thread; the remaining primitives map
// onto the concurrent::thread building blocks.
use crate::io;
use crate::ptr;
use crate::thread::ThreadInit;
use crate::time::Duration;
use concurrent::thread;
use syscall::{SystemCall, syscall};

pub struct Thread {
    id: usize,
}

// d3os ignores the requested stack size and gives every user thread a demand-paged
// 1 GiB stack, so this value is only the std-side default.
pub const DEFAULT_MIN_STACK_SIZE: usize = 1 << 20;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(_stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        // leak the boxed ThreadInit into a single thin pointer and pass it as the
        // trampoline's argument; the kernel forwards it to thread_start in rdi.
        let init_ptr = Box::into_raw(init);

        match syscall(SystemCall::ThreadCreate, &[thread_start as *const () as usize, init_ptr.expose_provenance()]) {
            Ok(id) => Ok(Thread { id }),
            Err(_) => {
                // the thread did not start, so the box was not consumed: reclaim it.
                drop(unsafe { Box::from_raw(init_ptr) });
                Err(io::Error::new(io::ErrorKind::Other, "create failed"))
            }
        }
    }

    pub fn join(self) {
        let _ = syscall(SystemCall::ThreadJoin, &[self.id]);
    }
}

// entry point of every spawned thread. runs on the new thread with init_ptr in rdi.
extern "sysv64" fn thread_start(init_ptr: usize) {
    // set up the thread environment (fs base) before any tls access, just like
    // concurrent::thread::kickoff_user_thread does for native d3os threads.
    thread::init_thread_environment();

    // rebuild the box from the raw pointer new() handed over, then run the closure
    let init = unsafe { Box::from_raw(ptr::with_exposed_provenance_mut::<ThreadInit>(init_ptr)) };
    let rust_start = init.init();
    rust_start();

    // thread_start has no caller to return to on d3os, so terminate the thread
    // explicitly instead of returning.
    thread::exit();
}

pub fn current_os_id() -> Option<u64> {
    thread::current().map(|current_thread| current_thread.id() as u64)
}

pub fn yield_now() {
    thread::switch();
}

pub fn sleep(duration: Duration) {
    // concurrent::thread::sleep works in whole milliseconds; round up so we never
    // sleep less than requested (mirrors the hermit backend).
    let milliseconds = duration.as_millis() + if duration.subsec_nanos() % 1_000_000 > 0 { 1 } else { 0 };
    let milliseconds = usize::try_from(milliseconds).unwrap_or(usize::MAX);
    thread::sleep(milliseconds);
}