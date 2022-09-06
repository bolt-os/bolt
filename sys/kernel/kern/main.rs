/*
 * Copyright (c) 2022 xvanc <xvancm@gmail.com>
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

//! Bolt Kernel

#![no_std]
#![no_main]
#![feature(
    default_alloc_error_handler,
    prelude_import,
    allocator_api,                  // https://github.com/rust-lang/rust/issues/32838
    asm_const,                      // https://github.com/rust-lang/rust/issues/93332
    asm_sym,                        // https://github.com/rust-lang/rust/issues/93333
    const_for,                      // https://github.com/rust-lang/rust/issues/87575
    const_mut_refs,                 // https://github.com/rust-lang/rust/issues/57349
    const_ptr_offset_from,          // "not yet stable"
    const_refs_to_cell,             // https://github.com/rust-lang/rust/issues/80384
    const_trait_impl,               // https://github.com/rust-lang/rust/issues/67792
    custom_test_frameworks,         // https://github.com/rust-lang/rust/issues/50297
    let_else,                       // https://github.com/rust-lang/rust/issues/87335
    naked_functions,                // https://github.com/rust-lang/rust/issues/32408
    nonnull_slice_from_raw_parts,   // https://github.com/rust-lang/rust/issues/71941
    once_cell,                      // https://github.com/rust-lang/rust/issues/74465
    pointer_is_aligned,             // https://github.com/rust-lang/rust/issues/96284
    strict_provenance,              // https://github.com/rust-lang/rust/issues/95228
    thread_local,                   // https://github.com/rust-lang/rust/issues/29594
)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test::run_tests)]
#![warn(clippy::cargo, clippy::pedantic, clippy::undocumented_unsafe_blocks)]
#![deny(
    clippy::semicolon_if_nothing_returned,
    clippy::debug_assert_with_mut_call,
    clippy::float_arithmetic
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::enum_glob_use,
    clippy::inline_always,
    clippy::items_after_statements,
    clippy::must_use_candidate,
    clippy::unreadable_literal,
    clippy::wildcard_imports
)]
#![forbid(clippy::inline_asm_x86_att_syntax)]

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(u64::BITS == usize::BITS);
};

extern crate alloc;
extern crate self as bolt;

mod prelude {
    pub use core::prelude::rust_2021::*;

    pub use alloc::{
        borrow::ToOwned,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    pub use core::arch::{asm, global_asm};

    pub use bolt::{include_asm, print, println, size_of};
    pub use boltk_macros::{asm_export, static_assert};

    pub use memoffset::offset_of;
}

use core::fmt::Write;

#[prelude_import]
#[allow(unused_imports)]
use bolt::prelude::*;

#[cfg_attr(target_arch = "x86_64", path = "../arch/x86_64/src/mod.rs")]
#[cfg_attr(target_arch = "riscv64", path = "../arch/riscv/src/mod.rs")]
mod arch;

mod kalloc;
mod logger;
mod panic;
mod pmm;
mod test;
mod trap;
mod utils;
mod vm;

pub struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        arch::evil_putstr(s);
        Ok(())
    }
}

pub fn __print(args: core::fmt::Arguments) {
    use spin::mutex::SpinMutex;
    static WRITER: SpinMutex<Writer> = SpinMutex::new(Writer);
    trap::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::__print(format_args!($($t)*)) };
}
#[macro_export]
macro_rules! println {
    ()          => { $crate::__print(format_args!("\n")) };
    ($($t:tt)*) => { $crate::__print(format_args!("{}\n", format_args!($($t)*))) };
}

pub fn kern_main() -> ! {
    todo!();
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mode {
    Kernel,
    User,
}
