use std::fs::OpenOptions;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

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

    // ------ fs: full round-trip + edge cases through std::fs ------
    // create -> write -> write -> seek -> read, then seek variants,
    // create_new, a read-only reopen, and the high-level std::fs helpers.
    let path = "/stdlibtest.txt";
    match OpenOptions::new().read(true).write(true).create(true).open(path) {
        Ok(mut file) => {
            // sequential writes advance the position
            match file.write(b"Hello, World!") {
                Ok(bytes_written) => println!("wrote {} bytes", bytes_written),
                Err(error) => println!("write failed: {:?}", error),
            }
            match file.write(b" I try to write into a file called stdlibtest.txt!") {
                Ok(bytes_written) => println!("wrote {} bytes", bytes_written),
                Err(error) => println!("write failed: {:?}", error),
            }

            // seek back to the start and read everything
            match file.seek(SeekFrom::Start(0)) {
                Ok(position) => println!("seek to {}", position),
                Err(error) => println!("seek failed: {:?}", error),
            }
            let mut buffer = [0u8; 512];
            match file.read(&mut buffer) {
                Ok(bytes_read) => {
                    let content = String::from_utf8_lossy(&buffer[..bytes_read]);
                    println!("read {} bytes from file: {:?}", bytes_read, content);
                }
                Err(error) => println!("reading file failed: {:?}", error),
            }

            // seek to the end (position == file size)
            match file.seek(SeekFrom::End(0)) {
                Ok(position) => println!("seek End(0) -> {}", position),
                Err(error) => println!("seek End(0) failed: {:?}", error),
            }
            // current position (maps to tell / seek(Current(0)))
            match file.stream_position() {
                Ok(position) => println!("stream_position -> {}", position),
                Err(error) => println!("stream_position failed: {:?}", error),
            }
            // absolute seek, then a short read (expected "World")
            let _ = file.seek(SeekFrom::Start(7));
            let mut small = [0u8; 5];
            match file.read(&mut small) {
                Ok(bytes_read) => println!("read {} bytes @7: {:?}",
                    bytes_read, String::from_utf8_lossy(&small[..bytes_read])),
                Err(error) => println!("read @7 failed: {:?}", error),
            }
            // relative forward seek
            match file.seek(SeekFrom::Current(3)) {
                Ok(position) => println!("seek Current(+3) -> {}", position),
                Err(error) => println!("seek Current(+3) failed: {:?}", error),
            }
            // negative offset is not expressible on d3os -> must be rejected, not crash
            match file.seek(SeekFrom::Current(-1)) {
                Ok(position) => println!("seek Current(-1): unexpected ok -> {}", position),
                Err(error) => println!("seek Current(-1): correctly rejected ({:?})", error),
            }
        }
        Err(error) => println!("opening file failed: {:?}", error),
    }

    // create_new is exclusive: first call creates, second must fail (EEXIST)
    match OpenOptions::new().write(true).create_new(true).open("/new.txt") {
        Ok(_) => println!("create_new /new.txt: created"),
        Err(error) => println!("create_new /new.txt: {:?} (already existed this boot?)", error),
    }
    match OpenOptions::new().write(true).create_new(true).open("/new.txt") {
        Ok(_) => println!("create_new /new.txt again: unexpected ok (should fail!)"),
        Err(error) => println!("create_new /new.txt again: correctly failed ({:?})", error),
    }

    // reopen read-only (no create) and read the first bytes
    match OpenOptions::new().read(true).open(path) {
        Ok(mut file) => {
            let mut head = [0u8; 13];
            match file.read(&mut head) {
                Ok(bytes_read) => println!("read-only reopen: {:?}",
                    String::from_utf8_lossy(&head[..bytes_read])),
                Err(error) => println!("read-only read failed: {:?}", error),
            }
        }
        Err(error) => println!("read-only reopen failed: {:?}", error),
    }

    // high-level helpers: File::create + write_all, then File::open + read_to_end
    let message = b"testing std::fs::write and std::fs::read_to_string";
    match std::fs::write("/hl.txt", message) {
        Ok(()) => println!("std::fs::write ok ({} bytes)", message.len()),
        Err(error) => println!("std::fs::write failed: {:?}", error),
    }
    match std::fs::read_to_string("/hl.txt") {
        Ok(content) => println!("std::fs::read_to_string -> {:?}", content),
        Err(error) => println!("std::fs::read_to_string failed: {:?}", error),
    }

    println!("FS TEST PASSED");

    // ------ thread ------
    // spawn a thread that captures a value and returns a result; join must
    // hand that result back (proves the closure + captured state + return path)
    let base = 100;
    let handle = thread::spawn(move || base + 23);
    let value = handle.join().unwrap();
    println!("spawn/join returned {} (expected 123)", value);
    assert_eq!(value, 123);

    // several threads share one counter through an Arc; after joining all of
    // them every increment must be visible (proves join really waits)
    let counter = Arc::new(AtomicUsize::new(0));
    let thread_count: usize = 4;
    let increments: usize = 1000;
    let mut handles = Vec::new();
    for _ in 0..thread_count {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..increments {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for handle in handles {
        let _ = handle.join();
    }
    let total = counter.load(Ordering::Relaxed);
    println!("counter = {} (expected {})", total, thread_count * increments);
    assert_eq!(total, thread_count * increments);

    // sleep and yield_now must just return cleanly
    thread::yield_now();
    thread::sleep(Duration::from_millis(10));
    println!("yield_now + sleep(10ms) ok");

    // d3os delegates available_parallelism to unsupported, so this always returns
    // Err ("unknown"); the Ok arm stays as a defensive fallback
    match thread::available_parallelism() {
        Ok(count) => println!("available_parallelism = {}", count),
        Err(error) => println!("available_parallelism = unknown ({:?})", error),
    }
    
    println!("THREAD TEST PASSED");
}