// This code has been partially generated with AI; future adjustments might be required
#![allow(dead_code)]

#[allow(unused_imports)]
use concurrent::{thread};
use core::ptr;

// These type aliases are correct for the std API:
pub type Key = usize;
pub type Dtor = unsafe extern "C" fn(*mut u8);

// Very dumb static tables (single-thread only).
const MAX_KEYS: usize = 128;

// Destructor functions associated with each key
static mut DTORS: [Option<Dtor>; MAX_KEYS] = [None; MAX_KEYS];
// Values associated with each key
static mut VALUES: [*mut u8; MAX_KEYS] = [ptr::null_mut(); MAX_KEYS];

static mut NEXT_KEY: usize = 0;

////////////////////////////////////////////////////////////////////////////////////////

// Create a TLS key with optional destructor
pub fn create(dtor: Option<Dtor>) -> Key {
    unsafe {
        let key = NEXT_KEY;
        if key >= MAX_KEYS {
            panic!("out of TLS keys");
        }
        NEXT_KEY += 1;

        // Safe access to statics requires unsafe blocks:
        DTORS[key] = dtor;
        VALUES[key] = ptr::null_mut();

        key
    }
}

// Destroy a TLS key (does not reclaim it)
pub unsafe fn destroy(key: Key) {
    let idx = key as usize;
    if idx >= MAX_KEYS {
        panic!("invalid TLS key");
    }

    unsafe {
        DTORS[idx] = None;
        VALUES[idx] = ptr::null_mut();
    }

    //concurrent.remove_tls_value(key); //<--- is that needed without key reclaiming?
}

// Run destructors on thread exit (single-thread bootstrap stub)
pub unsafe fn destroy_tls() {
    unsafe {
        for i in 0..MAX_KEYS {
            let val = VALUES[i];
            if !val.is_null() {
                if let Some(dtor) = DTORS[i] {
                    dtor(val);
                }
                VALUES[i] = ptr::null_mut();
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////

// Set the TLS value for a key
pub unsafe fn set(key: Key, val: *mut u8) {
    /*let idx = key as usize;
    if idx >= MAX_KEYS {
        panic!("invalid TLS key");
    }*/

    // Access to static mut = must be in unsafe block
    /*unsafe {
        VALUES[idx] = val;
    }*/

    //let mut thread = thread::current().unwrap();
    thread::set_tls_value(key, val);
}

// Get the TLS value for a key
pub unsafe fn get(key: Key) -> *mut u8 {
    /*let idx = key as usize;
    if idx >= MAX_KEYS {
        panic!("invalid TLS key");
    }*/

    //unsafe { VALUES[idx] }

    /*let mut thread = thread::current().unwrap();
    if let Some(val) = thread.get_tls_value(idx) {
        let boxed = Box::new(val); //allocate on heap
        let ptr: *mut u8 = Box::into_raw(boxed); //take ownership
        return ptr;
    }
    core::ptr::null_mut() //else null*/
    thread::get_tls_value(key)
}