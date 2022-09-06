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

#[macro_export]
macro_rules! asm_export {
    ($sym:ident) => {
        ::core::arch::global_asm! {
            ::core::concat!(".global ", stringify!($sym)),
            ::core::concat!(".set    ", stringify!($sym), ", {}"),
            const $sym,
        }
    };
    ($sym:ident = $val:expr) => {
        ::core::arch::global_asm! {
            ::core::concat!(".global ", stringify!($sym)),
            ::core::concat!(".set    ", stringify!($sym), ", {}"),
            const $val,
        }
    };
}

#[macro_export]
macro_rules! include_asm {
    ($file:expr $(, options($($options:tt)*))?) => {
        ::core::arch::global_asm!(
            ::core::include_str!($file),
            options(
                raw$(,
                $($options)*)?
            )
        );
    };
}

#[macro_export]
macro_rules! size_of {
    ($t:ty) => {
        ::core::mem::size_of::<$t>()
    };
}

#[macro_export]
macro_rules! count_arguments {
    ($($x:tt),*$(,)?) => { 0 $( + $crate::count_arguments!(@count $x) )* };
    (@count $x:tt) => { 1 };
}

/// Run some code only one time.
///
/// This is particularly useful in initialization code, where some things only
/// need to be done by the first CPU that executes it.
///
/// All other threads will block waiting for the first thread to finish.
#[macro_export]
macro_rules! run_once {
    { $($critical_section:tt)* } => {{
        use ::core::sync::atomic::{AtomicU32, Ordering};

        const NOT_RUN: u32 = 0;
        const RUNNING: u32 = 1;
        const HAS_RUN: u32 = 2;

        static RUN_STATE: AtomicU32 = AtomicU32::new(NOT_RUN);

        loop {
            match RUN_STATE.compare_exchange_weak(
                NOT_RUN,
                RUNNING,
                Ordering::SeqCst,
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    {
                        $($critical_section)*
                    }
                    RUN_STATE.store(HAS_RUN, Ordering::SeqCst);
                    break;
                }
                Err(RUNNING) => ::core::hint::spin_loop(),
                Err(HAS_RUN) => break,
                _ => ::core::unreachable!(),
            }
        }
    }};
}
