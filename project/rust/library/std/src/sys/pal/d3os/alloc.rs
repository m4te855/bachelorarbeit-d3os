// FIXME(static_mut_refs): taken from sys/alloc/xous.rs
#![allow(static_mut_refs)]

// Global allocator (std::alloc::System) for the D3OS target, built in three layers:
//   1. OS backend (`D3osSystem`): coarse, page-aligned memory from the kernel;
//      the only layer that sources memory from D3OS.
//   2. Engine (`Dlmalloc`): carves those blocks into the chunks `Box`, `Vec`, ...
//      actually ask for, and reuses freed ones.
//   3. std entry point (`impl GlobalAlloc for System`), serialised by a lock.
//
// Layers 2 and 3 are modelled on library/std/src/sys/alloc/xous.rs,
// layer 1 on dlmalloc-rs/src/xous.rs:
// https://github.com/alexcrichton/dlmalloc-rs/blob/0.2.10/src/xous.rs
// Layer 1 exists because dlmalloc ships backends only for the systems it knows, and
// D3OS is not among them. D3OS-specific: memory comes from the `MapMemory` syscall,
// and the lock yields via `ThreadSwitch` instead of Xous' `do_yield()`.

use crate::alloc::{GlobalAlloc, Layout, System};
use crate::ptr;
use crate::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use dlmalloc::{Allocator, Dlmalloc};
use syscall::{SystemCall, syscall};

// Heap window identical to the values in the D3OS runtime crate (runtime/src/env.rs),
// so std uses exactly the region D3OS intends for user-space heaps.
const HEAP_START: usize = 63 * 1024 * 1024 * 1024 * 1024; // 63 TiB
const HEAP_SIZE: usize = 1024 * 1024 * 1024 * 1024; // 1 TiB

// Layer 1: supplies dlmalloc with coarse kernel memory. Counterpart of `System` in
// dlmalloc-rs/src/xous.rs, with `MapMemory` in place of the Xous heap syscall;
// renamed because `System` is taken by the std allocator type imported above.
struct D3osSystem;

// Set once the region is mapped, so `MapMemory` is issued on the first call only.
static HEAP_MAPPED: AtomicBool = AtomicBool::new(false);
// Bump pointer: start of the next block to hand out, only ever grows.
static NEXT: AtomicUsize = AtomicUsize::new(HEAP_START);

unsafe impl Allocator for D3osSystem {
    // Returns (pointer, usable size, flags); a null pointer signals failure.
    // Reachable only through `HEAP`, which the `lock` module below guards, so the
    // backend never runs concurrently and `Ordering::Relaxed` suffices throughout.
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        // Round up to a multiple of the page size (4 KiB); 0 -> 4096.
        let size = if size == 0 {
            4096
        } else if size & 4095 == 0 {
            size
        } else {
            size + (4096 - (size & 4095))
        };

        // Reserve the whole region on the first call. The kernel only sets up a VMA
        // and fills page tables on demand (`sys_map_memory` in kernel/src/syscall/
        // sys_vmem.rs), so reserving 1 TiB costs next to nothing. Xous grows its heap
        // on every backend call instead; D3OS maps once and sub-allocates below.
        if !HEAP_MAPPED.load(Ordering::Relaxed) {
            if syscall(SystemCall::MapMemory, &[HEAP_START, HEAP_SIZE]).is_err() {
                return (ptr::null_mut(), 0, 0);
            }
            // Set only after success, so a failed first attempt can retry.
            HEAP_MAPPED.store(true, Ordering::Relaxed);
        }

        // Hand out the next block from the reserved region (bump allocation).
        // `fetch_add` returns the previous value.
        let addr = NEXT.fetch_add(size, Ordering::Relaxed);
        // `NEXT` may end up past the limit on the failing call, which is harmless: later
        // calls overshoot the same way and return null. No invalid pointer escapes.
        if addr + size > HEAP_START + HEAP_SIZE {
            return (ptr::null_mut(), 0, 0); // region exhausted
        }
        // A kernel mapping, not an existing allocation, so there is no pointer to
        // derive provenance from; strict-provenance form of `addr as *mut u8`.
        (ptr::with_exposed_provenance_mut::<u8>(addr), size, 0)
    }

    // Memory is never returned to the kernel; `remap` yields null, the others false,
    // marking them unsupported. Nothing leaks: dlmalloc reuses freed chunks from its
    // own free lists and only calls `alloc` above when it needs more.
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
    // Conservative: we do not assume the kernel hands out zeroed pages, so dlmalloc
    // zeroes `calloc` itself (the Xous backend returns `true` here).
    fn allocates_zeros(&self) -> bool {
        false
    }
    // Matches `PAGE_SIZE` in the D3OS kernel.
    fn page_size(&self) -> usize {
        4096
    }
}

// Layers 2 + 3, counterpart of library/std/src/sys/alloc/xous.rs. The one real
// change is the constructor: xous.rs uses `Dlmalloc::new()`, which picks dlmalloc's
// built-in backend for the target, and there is none for D3OS. `new_with_allocator`
// injects ours, and that is the only reason layer 1 exists.
static mut HEAP: Dlmalloc<D3osSystem> = Dlmalloc::new_with_allocator(D3osSystem);

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _lock = lock::lock();
        // SAFETY: `HEAP` is touched only under the lock (exclusive access). A valid layout
        // is the caller's obligation under the `GlobalAlloc` contract.
        unsafe { HEAP.malloc(layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _lock = lock::lock();
        // SAFETY: as in `alloc`; dlmalloc does the zeroing, see `allocates_zeros`.
        unsafe { HEAP.calloc(layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _lock = lock::lock();
        // SAFETY: as in `alloc`; the caller guarantees `ptr` and `layout` match.
        unsafe { HEAP.free(ptr, layout.size(), layout.align()) }
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _lock = lock::lock();
        // SAFETY: as in `dealloc`; dlmalloc grows the chunk in place where it can.
        unsafe { HEAP.realloc(ptr, layout.size(), layout.align(), new_size) }
    }
}

// Test-and-set lock: dlmalloc keeps mutable bookkeeping in `HEAP`, so only one thread
// may be inside it at a time. Counterpart of the `lock` module in sys/alloc/xous.rs
// with one substitution, `ThreadSwitch` for `do_yield()`. Not reentrant: allocating
// while holding it would deadlock.
mod lock {
    use crate::sync::atomic::AtomicI32;
    use crate::sync::atomic::Ordering::{Acquire, Release};
    use syscall::{SystemCall, syscall};

    // 0 = free, 1 = held
    static LOCKED: AtomicI32 = AtomicI32::new(0);

    pub struct DropLock;

    pub fn lock() -> DropLock {
        loop {
            // Set to 1 and report the previous value; 0 means it was free and is now
            // ours. `Acquire` pairs with the `Release` in `drop`.
            if LOCKED.swap(1, Acquire) == 0 {
                return DropLock;
            }
            // Held elsewhere: hand back the CPU instead of spinning. The thread stays
            // runnable and retries at the top of the loop once rescheduled.
            let _ = syscall(SystemCall::ThreadSwitch, &[]);
        }
    }

    impl Drop for DropLock {
        fn drop(&mut self) {
            // Store 0 to free the lock. `Release` publishes our `HEAP` writes to the next
            // acquirer; the previous value must be 1 (the lock was held).
            let r = LOCKED.swap(0, Release);
            debug_assert_eq!(r, 1);
        }
    }
}