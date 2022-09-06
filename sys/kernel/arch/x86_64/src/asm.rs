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

use crate::vm::VirtAddr;

use super::cpu::Rflags;

pub fn read_cr2() -> usize {
    let cr0;
    unsafe {
        asm!("mov {:r}, cr2", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    cr0
}

pub unsafe fn write_cr2(value: usize) {
    asm!("mov cr2, {:r}", in(reg) value, options(nomem, nostack, preserves_flags));
}

pub unsafe fn write_cr3(value: usize) {
    // NOTE: The CR3 register contains the physical address of the root
    // page table. Writes to this register change the current address space
    // and flush the entire TLB, so we must not specify the `nomem` option.
    // This informs the compiler that the asm clobbers memory, so it will
    // flush cached values from registers before executing it, and reload
    // them after.
    asm!("mov cr3, {:r}", in(reg) value, options(nostack, preserves_flags));
}

pub unsafe fn invlpg(virt: VirtAddr) {
    asm!("invlpg {:r}", in(reg) virt.to_usize(), options(nostack, preserves_flags));
}

pub unsafe fn cli() {
    asm!("cli", options(nomem, nostack, preserves_flags));
}

pub unsafe fn push_cli() -> bool {
    let bits: usize;
    asm!(
        "pushfq; cli; pop {:r}",
        out(reg) bits,
        options(nomem, nostack, preserves_flags)
    );
    Rflags::new_unchecked(bits).contains(Rflags::IF)
}

/// # Safety
pub unsafe fn push_stac() -> bool {
    let bits: usize;
    asm!(
        "pushfq; stac; pop {:r}",
        out(reg) bits,
        options(nomem, nostack, preserves_flags)
    );
    Rflags::new_unchecked(bits).contains(Rflags::AC)
}

pub unsafe fn sti() {
    asm!("sti", options(nomem, nostack, preserves_flags));
}

pub unsafe fn clac() {
    asm!("clac", options(nomem, nostack, preserves_flags));
}
