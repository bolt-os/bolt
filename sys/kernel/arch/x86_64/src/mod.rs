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

use core::mem::MaybeUninit;
use crate::cpu::Cpu;

mod cpu;
mod msr;
mod trap;

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") };
    }
}

unsafe fn port3f8_write(s: &str) {
    asm!(
        "rep outsb",
        in("rsi") s.as_ptr(),
        in("ecx") s.len(),
        in("edx") 0x3f8,
        options(nostack, preserves_flags),
    );
}

static mut CPU0_STORAGE: MaybeUninit<Cpu> = MaybeUninit::uninit();

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    port3f8_write("hello, world!\r\n");

    let this_cpu = CPU0_STORAGE.as_mut_ptr();
    cpu::early_init(this_cpu);

    port3f8_write("hello, again!\r\n");

    hcf();
}
