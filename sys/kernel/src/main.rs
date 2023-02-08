#![no_std]
#![no_main]
// Unstable Features
#![feature(
    prelude_import,
    custom_test_frameworks,                 // https://github.com/rust-lang/rust/issues/50297
)]
// Custom Test Framework
#![reexport_test_harness_main = "test_main"]
#![test_runner(test::run)]

#[cfg(notyet)]
extern crate alloc;

#[prelude_import]
#[allow(unused_imports)]
use self::prelude::*;
mod prelude {
    // Bring back core's prelude.
    pub use core::{
        // Bring back `asm!`. (i'm still bitter)
        arch::{asm, global_asm},
        // prelude::*,
        prelude::rust_2021::*,
    };

    // Items from `alloc` usually included by `std`'s prelude.
    #[cfg(notyet)]
    pub use alloc::{
        borrow::ToOwned,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
}

mod arch;
mod panic;
mod test;

/// Main machine-independent kernel entry point
pub fn main() {}
