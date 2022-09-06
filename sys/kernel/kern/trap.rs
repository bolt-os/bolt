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

use bolt::arch::trap as arch_trap;
pub use bolt::arch::trap::without_interrupts;

#[repr(transparent)]
pub struct Vector(usize);

impl Vector {
    pub fn is_exception(self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            self.0 < 32
        }
        #[cfg(target_arch = "riscv64")]
        {
            self.0 & (1 << (usize::BITS - 1)) == 0
        }
    }
}

struct TrapHandler {}

pub fn disable() {
    arch_trap::disable();
}

pub struct Exception {
    pub vector: usize,
    pub name: &'static str,
}

pub trait TrapFrameApi {
    fn is_exception(&self) -> bool;
    fn exception(&self) -> Option<&'static Exception>;
}

const _: () = {
    trait TrapFrameApiCheck: TrapFrameApi {}
    impl TrapFrameApiCheck for arch_trap::TrapFrame {}
};

pub extern "C" fn dispatch(tf: &mut arch_trap::TrapFrame) {
    log::info!("trap!\n{tf:#018x?}");

    if let Some(excpt) = tf.exception() {
        panic!("fatal exception: {}", excpt.name);
    }

    todo!("lmao");
}

pub fn register_handler() {}
