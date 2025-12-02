/*pub type Key = usize;
pub type Dtor = unsafe extern "C" fn(*mut u8);

//legt einen Wert im TLS unter einem bestimmten Schlüssel ab
pub unsafe fn set(_key: Key, _val: *mut u8) {
    todo!()
    //set_tls_value
}

#[allow(dead_code)]
//holt einen Wert, der unter einem bestimmten Schlüssel abgelegt wurde aus dem TLS
pub unsafe fn get(_key: Key) -> *mut u8 {
    todo!()
    //get_tls_value
}

//erzeugt einen Schlüssel
pub fn create(_dtor: Option<Dtor>) -> Key {
    todo!()
}

//zertört einen Schlüssel (ggf. Wert wieder freigeben)
pub unsafe fn destroy(_key: Key) {
    todo!()
}

#[allow(dead_code)]
//zerstört alle Schlüssel bei Beendigung des Threads
pub unsafe fn destroy_tls() {
    todo!()
}*/

//KI Müll

// library/std/src/sys/thread_local/key/myos.rs 

#![allow(dead_code)]

use core::ptr;
//use concurrent::*;

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

// Set the TLS value for a key
pub unsafe fn set(key: Key, val: *mut u8) {
    let idx = key as usize;
    if idx >= MAX_KEYS {
        panic!("invalid TLS key");
    }

    // Access to static mut = must be in unsafe block
    unsafe {
        VALUES[idx] = val;
    }

    //concurrent.set_tls_value(key, val);
}

// Get the TLS value for a key
pub unsafe fn get(key: Key) -> *mut u8 {
    let idx = key as usize;
    if idx >= MAX_KEYS {
        panic!("invalid TLS key");
    }

    unsafe { VALUES[idx] }

    //concurrent.get_tls_value(key);
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

    //concurrent.remove_tls_value(key);
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


