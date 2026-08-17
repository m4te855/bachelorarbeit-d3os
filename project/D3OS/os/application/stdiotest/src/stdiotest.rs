use std::io::{self, BufRead, Read, Write};

#[unsafe(no_mangle)]
pub fn main() {
    // ------ stdio (output) ------
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write(b"Writing directly with stdout!\n");
    print!("This has been written using the print!-macro!\n");
    println!("And this has been written using {}!", "println!");

    // ----- stdio (input) ------
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // reading a line using read_line
    println!("Type something and press Enter (read_line test):");
    let mut line = String::new();
    match handle.read_line(&mut line) {
        Ok(bytes_read) => println!("Read {} bytes with read_line: {:?}", bytes_read, line),
        Err(error) => println!("Reading with read_line failed: {:?}", error),
    }

    // interactive stdin test loop, press Enter on an empty line to quit
    println!("Input test loop, empty line quits:");
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
}