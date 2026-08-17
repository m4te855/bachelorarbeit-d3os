// Thread backend for D3OS. The spawn/trampoline structure is modelled on
// hermit.rs, the thinnest wrapper over a runtime crate: spawn goes through the
// raw ThreadCreate syscall so we can hand the boxed ThreadInit to the new
// thread. The remaining primitives map onto the concurrent::thread building
// blocks that native D3OS applications use.

// set_name and available_parallelism are not implemented here; sys/thread/mod.rs pulls those
// two out of unsupported.rs for this target (D3OS has no thread names or dedicated syscall).
use crate::io;
use crate::ptr;
use crate::thread::ThreadInit;
use crate::time::Duration;
use concurrent::thread;
use syscall::{SystemCall, syscall};

// Mirrors concurrent::thread::Thread: identified solely by ID.
pub struct Thread {
    id: usize,
}

// Re-exported by sys/thread/mod.rs and read by thread::Builder as the default stack size,
// though D3OS ignores it and always maps a fixed 1 GiB demand-paged stack. Value from hermit.rs.
pub const DEFAULT_MIN_STACK_SIZE: usize = 1 << 20;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements. Matches unsupported.rs:
    // `init` is used, while `_stack` is ignored (D3OS syscall takes no stack parameter).
    pub unsafe fn new(_stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        // D3OS expects a bare fn() entry point, so we split std's stateful closure into
        // a trampoline (`thread_start`) and a raw data pointer reconstructed via `Box::from_raw`.
        let init_ptr = Box::into_raw(init);

        // arg0 = trampoline, arg1 = ThreadInit pointer (forwarded by kernel to rdi via SysV ABI).
        // `expose_provenance` keeps the provenance recoverable by the trampoline.
        match syscall(SystemCall::ThreadCreate,
            &[thread_start as *const () as usize, init_ptr.expose_provenance()]) {
            Ok(id) => Ok(Thread { id }),
            Err(_) => {
                // Creation failed: reclaim the unconsumed Box (mirrors hermit.rs).
                drop(unsafe { Box::from_raw(init_ptr) });
                Err(io::Error::new(io::ErrorKind::Other, "create failed"))
            }
        }
    }

    pub fn join(self) {
        // Mirrors concurrent::thread::Thread::join (consumes `self` per std requirements).
        let _ = syscall(SystemCall::ThreadJoin, &[self.id]);
    }
}

// Entry point for spawned threads; `sysv64` matches the kernel's ABI passing `init_ptr` in rdi.
extern "sysv64" fn thread_start(init_ptr: usize) {
    // Set up the thread environment (fs base) before any tls access, just like
    // concurrent::thread::kickoff_user_thread does for native D3OS threads.
    thread::init_thread_environment();

    // Rebuild the box from the raw pointer new() handed over, then run the closure
    // (int -> pointer needs with_exposed_provenance_mut; std denies fuzzy_provenance_casts).
    let init = unsafe { Box::from_raw(ptr::with_exposed_provenance_mut::<ThreadInit>(init_ptr)) };
    // Registers thread handle for `thread::current()` and runs the user payload.
    let rust_start = init.init();
    rust_start();

    // Explicit exit (no caller to return to). NOTE: unlike hermit.rs, this backend
    // does not run TLS destructors yet.
    thread::exit();
}

// Adapts `thread::current()` (syscall ThreadId) to the `Option<u64>` expected by std.
pub fn current_os_id() -> Option<u64> {
    thread::current().map(|current_thread| current_thread.id() as u64)
}

// Yields CPU to another thread via `thread::switch()` (syscall ThreadSwitch).
pub fn yield_now() {
    thread::switch();
}

pub fn sleep(duration: Duration) {
    // concurrent::thread::sleep works in whole milliseconds; round up so we never
    // sleep less than requested (mirrors the hermit backend).
    let milliseconds = duration.as_millis() +
        if duration.subsec_nanos() % 1_000_000 > 0 { 1 } else { 0 };
    let milliseconds = usize::try_from(milliseconds).unwrap_or(usize::MAX);
    thread::sleep(milliseconds);
}