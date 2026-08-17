use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

#[unsafe(no_mangle)]
pub fn main() {
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
            // negative offset is not expressible on D3OS -> must be rejected, not crash
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
}