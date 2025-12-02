#![deny(unsafe_op_in_unsafe_fn)]

pub mod os;
pub mod pipe;
pub mod thread;
pub mod time;

pub mod start;
pub mod alloc;

//neu
//pub mod stdio;

mod common;
pub use common::*;