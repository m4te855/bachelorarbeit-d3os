// FIXME(static_mut_refs): taken from sys/alloc/xous.rs
#![allow(static_mut_refs)]

// Global allocator (std::alloc::System) for the D3OS target.
//
// Modeled on the Xous port of the standard library:
//   - std side  (GlobalAlloc + static + lock): library/std/src/sys/alloc/xous.rs
//   - OS backend (impl Allocator):             dlmalloc-rs/src/xous.rs https://github.com/alexcrichton/dlmalloc-rs/blob/0.2.10/src/xous.rs
// 
// D3OS-specific adaptations:
//   - the backend obtains memory via the `MapMemory` syscall instead of the
//     Xous heap syscall (see `D3osSystem::alloc`);
//   - on contention the lock yields via the `ThreadSwitch` syscall instead of
//     Xous' `do_yield()`.

use crate::alloc::{GlobalAlloc, Layout, System};
use crate::ptr;
use crate::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use dlmalloc::{Allocator, Dlmalloc};
use syscall::{SystemCall, syscall};

// Heap window in the virtual address space, identical to the values in the D3OS
// runtime crate (runtime/src/env.rs), which mirrors the kernel's consts.rs.
const HEAP_START: usize = 63 * 1024 * 1024 * 1024 * 1024; // 63 TiB
const HEAP_SIZE: usize = 1024 * 1024 * 1024 * 1024; // 1 TiB

// ---------------------------------------------------------------------------
// Layer 1: OS backend. Supplies dlmalloc with coarse, page-aligned memory
// obtained from the kernel. Counterpart of `System` in dlmalloc-rs/src/xous.rs,
// with the Xous heap syscall replaced by `MapMemory`.
// ---------------------------------------------------------------------------
struct D3osSystem;

// `true` once the heap region has been mapped (MapMemory runs exactly once).
static HEAP_MAPPED: AtomicBool = AtomicBool::new(false);
// Bump pointer: next free address to hand out within the mapped region.
static NEXT: AtomicUsize = AtomicUsize::new(HEAP_START);

unsafe impl Allocator for D3osSystem {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        // Round the request up to a multiple of the page size (4 KiB); 0 -> 4096.
        let size = if size == 0 {
            4096
        } else if size & 4095 == 0 {
            size
        } else {
            size + (4096 - (size & 4095))
        };

        // Reserve the whole heap region once. The kernel only sets up a VMA here;
        // physical frames are allocated lazily on page faults (demand paging), so
        // reserving 1 TiB up front is essentially free. This is the same
        // `MapMemory` call the D3OS runtime makes for no_std applications.
        if !HEAP_MAPPED.load(Ordering::Relaxed) {
            if syscall(SystemCall::MapMemory, &[HEAP_START, HEAP_SIZE]).is_err() {
                return (ptr::null_mut(), 0, 0);
            }
            // Set the flag only after success, so a failed first attempt can retry.
            HEAP_MAPPED.store(true, Ordering::Relaxed);
        }

        // Hand out the next block from the reserved region (bump allocation).
        let addr = NEXT.fetch_add(size, Ordering::Relaxed);
        if addr + size > HEAP_START + HEAP_SIZE {
            return (ptr::null_mut(), 0, 0); // region exhausted
        }
        (ptr::with_exposed_provenance_mut::<u8>(addr), size, 0)
    }

    // Coarse blocks are never returned to the kernel; dlmalloc reuses freed
    // memory internally and only calls back here for fresh memory.
    fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
        ptr::null_mut()
    }
    fn free_part(&self, _ptr: *mut u8, _old: usize, _new: usize) -> bool {
        false
    }
    fn free(&self, _ptr: *mut u8, _size: usize) -> bool {
        false
    }
    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }
    // Conservative: do not assume freshly mapped pages are zeroed, so dlmalloc
    // zeroes calloc allocations itself. Set to `true` if the D3OS kernel
    // guarantees zeroed pages (makes calloc faster).
    fn allocates_zeros(&self) -> bool {
        false
    }
    fn page_size(&self) -> usize {
        4096
    }
}

// ---------------------------------------------------------------------------
// Layers 2 + 3: the dlmalloc engine plus the `System` GlobalAlloc impl, guarded
// by a lock. Counterpart of library/std/src/sys/alloc/xous.rs, with our custom
// backend passed via `new_with_allocator`.
// ---------------------------------------------------------------------------
static mut HEAP: Dlmalloc<D3osSystem> = Dlmalloc::new_with_allocator(D3osSystem);

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _lock = lock::lock();
        // SAFETY: the lock grants unique, non-reentrant access to HEAP.
        unsafe { HEAP.malloc(layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _lock = lock::lock();
        unsafe { HEAP.calloc(layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _lock = lock::lock();
        unsafe { HEAP.free(ptr, layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _lock = lock::lock();
        unsafe { HEAP.realloc(ptr, layout.size(), layout.align(), new_size) }
    }
}

// Test-and-set lock guarding the single heap instance. Counterpart of the
// `lock` module in sys/alloc/xous.rs; on contention it yields to the scheduler
// via the `ThreadSwitch` syscall (the D3OS equivalent of Xous' `do_yield()`).
mod lock {
    use crate::sync::atomic::AtomicI32;
    use crate::sync::atomic::Ordering::{Acquire, Release};
    use syscall::{SystemCall, syscall};

    static LOCKED: AtomicI32 = AtomicI32::new(0);

    pub struct DropLock;

    pub fn lock() -> DropLock {
        loop {
            if LOCKED.swap(1, Acquire) == 0 {
                return DropLock;
            }
            // Lock is held by another thread: give the CPU to the scheduler.
            let _ = syscall(SystemCall::ThreadSwitch, &[]);
        }
    }

    impl Drop for DropLock {
        fn drop(&mut self) {
            let r = LOCKED.swap(0, Release);
            debug_assert_eq!(r, 1);
        }
    }
}