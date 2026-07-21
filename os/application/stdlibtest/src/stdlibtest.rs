use std::io::{self, BufRead, Read, Write};

// A type whose 4096-byte alignment far exceeds the 8 the old stub allocator allowed.
#[repr(align(4096))]
struct Aligned([u8; 4096]);

#[unsafe(no_mangle)]
pub fn main() {
    // ------ stdio (output) ------
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write(b"Writing directly with stdout!\n");
    print!("This has been written using the print!-macro!\n");
    println!("And this has been written using {}!", "println!");

    // ----- stdio (input) ------
    // reading from stdin directly
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    println!("Type something and press Enter:");
    let mut buffer = [0u8; 128];
    match handle.read(&mut buffer) {
        Ok(bytes_read) => {
            let input = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Read {} bytes from stdin: {:?}", bytes_read, input);
        }
        Err(error) => println!("Reading from stdin failed: {:?}", error),
    }
    // reading a line using read_line
    println!("Type something and press Enter (read_line test):");
    let mut line = String::new();
    match handle.read_line(&mut line) {
        Ok(bytes_read) => println!("Read {} bytes with read_line: {:?}", bytes_read, line),
        Err(error) => println!("Reading with read_line failed: {:?}", error),
    }
/* 
    // interactive stdin test loop, press Enter on an empty line to quit
    println!("Input test loop, empty line quits:");
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = [0u8; 128];
    loop {
        match handle.read(&mut buffer) {
            Ok(bytes_read) => {
                let input = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("Read {} bytes from stdin: {:?}", bytes_read, input);
                if input == "\n" {
                    break;
                }
            }
            Err(error) => {
                println!("Reading from stdin failed: {:?}", error);
                break;
            }
        }
    }
*/
    // ------ heap ------
    // --- heap: large allocation (beyond the old 64 KiB stub buffer) ---
    let big: Vec<u8> = vec![0; 1024 * 1024];
    println!("1 MiB allocation ok (len = {})", big.len());
    drop(big);

    // --- heap: many allocate/free cycles (proves dealloc reclaims) ---
    let mut sum: u64 = 0;
    for i in 0..1000 {
        let mut v: Vec<u8> = vec![0; 64 * 1024];
        v[i] = 1;              
        sum += v[i] as u64;     // prevent dead-allocation elimination
    }
    println!("1000 x 64 KiB alloc/free cycles ok (dealloc reclaims, checksum {})", sum);

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