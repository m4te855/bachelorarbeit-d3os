// This code has been partially generated with AI; future adjustments might be required
#![allow(dead_code)]

#[allow(unused_imports)]
use concurrent::{thread};

// these type aliases are correct for the std API
pub type Key = usize;
pub type Dtor = unsafe extern "C" fn(*mut u8);

// very dumb static tables
const MAX_KEYS: usize = 1024;

// destructor functions associated with each key
static mut DTORS: [Option<Dtor>; MAX_KEYS] = [None; MAX_KEYS];

// simple counter for next key
static mut NEXT_KEY: usize = 0;

//////////////////////////////////////////////////////////////////////

// create a TLS key with optional destructor
pub fn create(dtor: Option<Dtor>) -> Key {
    unsafe {
        let key = NEXT_KEY;
        if key >= MAX_KEYS {
            panic!("out of TLS keys");
        }
        NEXT_KEY += 1;

        DTORS[key] = dtor;
        key
    }
}

// destroy a TLS key (does not reclaim it)
pub unsafe fn destroy(key: Key) {
    if key >= MAX_KEYS {
        panic!("invalid TLS key");
    }

    unsafe {
        DTORS[key] = None;
    }

    // if keys were reclaimed, then a function "thread::delete_tls_value" could be called here
}

// run destructors on thread exit (stub)
pub unsafe fn destroy_tls() {
    /*unsafe {
        for i in 0..MAX_KEYS {
            let val = VALUES[i];
            if !val.is_null() {
                if let Some(dtor) = DTORS[i] {
                    dtor(val);
                }
            }
        }
    }*/
}

//////////////////////////////////////////////////////////////////////

// set the TLS value for a key
pub unsafe fn set(key: Key, val: *mut u8) {
    if key >= MAX_KEYS {
        panic!("invalid TLS key");
    }

    thread::set_tls_value(key, val);
}

// get the TLS value for a key
pub unsafe fn get(key: Key) -> *mut u8 {
    if key >= MAX_KEYS {
        panic!("invalid TLS key");
    }
    thread::get_tls_value(key)
}