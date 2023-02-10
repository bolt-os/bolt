/*
 * Copyright (c) 2023 xvanc <xvancm@gmail.com>
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its contributors
 *    may be used to endorse or promote products derived from this software without
 *    specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 */

#![no_std]
#![no_main]
// Unstable Features
#![feature(
    prelude_import,
    asm_const,                              // https://github.com/rust-lang/rust/issues/93332
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
mod cpu;
mod panic;
mod test;
mod trap;

/// Main machine-independent kernel entry point
pub fn main() {}
