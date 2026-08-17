use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[unsafe(no_mangle)]
pub fn main() {
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

    // D3OS delegates available_parallelism to unsupported, so this always returns
    // Err ("unknown"); the Ok arm stays as a defensive fallback
    match thread::available_parallelism() {
        Ok(count) => println!("available_parallelism = {}", count),
        Err(error) => println!("available_parallelism = unknown ({:?})", error),
    }
    
    println!("THREAD TEST PASSED");
}