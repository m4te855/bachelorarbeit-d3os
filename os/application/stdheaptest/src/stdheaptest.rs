// A type whose 4096-byte alignment far exceeds the 8 the old stub allocator allowed.
#[repr(align(4096))]
struct Aligned([u8; 4096]);

#[unsafe(no_mangle)]
pub fn main() {
    // ------ heap ------
    // --- heap: large allocation (beyond the old 64 KiB stub buffer) ---
    let big: Vec<u8> = vec![0; 1024 * 1024];
    println!("1 MiB allocation ok (len = {})", big.len());
    drop(big);

    // --- heap: many allocate/free cycles (proves dealloc reclaims) ---
    let mut sum: u64 = 0;
    for i in 0..10000 {
        let mut v: Vec<u8> = vec![0; 64 * 1024];
        v[i] = 1;              
        sum += v[i] as u64;     // prevent dead-allocation elimination
    }
    println!("10000 x 64 KiB alloc/free cycles ok (dealloc reclaims, checksum {})", sum);

    // --- heap: over-aligned allocation (old stub rejected align > 8) ---
    let a = Box::new(Aligned([1; 4096]));
    assert_eq!(&*a as *const Aligned as usize % 4096, 0);
    println!("4096-byte aligned allocation ok (first byte = {})", a.0[0]);
    drop(a);

    // --- heap: many live allocations + Vec growth (realloc) + partial/bulk free ---
    let mut live: Vec<Box<[u8; 1024]>> = Vec::new();
    for _ in 0..1000 {
        live.push(Box::new([0; 1024]));
    }
    println!("1000 simultaneous live allocations ok (Vec realloc, {} live)", live.len());
    live.truncate(500);
    println!("partial free ok ({} live)", live.len());
    drop(live);
    println!("HEAP TEST PASSED");
}