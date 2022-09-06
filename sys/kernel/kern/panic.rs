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

pub fn hcf() -> ! {
    bolt::trap::disable();
    loop {
        core::hint::spin_loop();
    }
}

use core::sync::atomic::{AtomicBool, Ordering};

use spin::RwLock;

static KERNEL_ELF: RwLock<Option<Vec<u8>>> = RwLock::new(None);

pub fn register_executable(elf: Vec<u8>) -> Result<(), ()> {
    let mut exec = KERNEL_ELF.write();

    if exec.is_none() {
        *exec = Some(elf);
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(target_arch = "x86_64")]
fn get_backtrace() -> Vec<usize> {
    #[repr(C)]
    struct Frame {
        next: *const Frame,
        retaddr: usize,
    }
    let mut addrs = vec![];
    let mut fp: *const Frame;
    unsafe {
        asm!("mov {}, rbp", out(reg) fp);
        while !fp.is_null() {
            addrs.push((*fp).retaddr);
            fp = (*fp).next;
        }
    }
    addrs
}
#[cfg(target_arch = "riscv64")]
fn get_backtrace() -> Vec<usize> {
    #[repr(C)]
    struct Frame {
        next: *const Frame,
        retaddr: usize,
    }

    let mut addrs = vec![];
    let mut fp: *const Frame;

    unsafe {
        asm!("mv {0}, fp", out(reg) fp);
        fp = fp.sub(1);
        while !fp.is_null() {
            let addr = (*fp).retaddr;
            if addr == 0 {
                break;
            }
            addrs.push(addr);
            fp = (*fp).next.sub(1);
        }
    }

    addrs
}

#[inline(never)]
fn trace_stack() {
    let elf_guard = KERNEL_ELF.read();
    let elf = elf_guard.as_ref().and_then(|data| elf::Elf::new(data).ok());

    println!("----- STACK TRACE -----");
    for (count, addr) in get_backtrace().into_iter().enumerate() {
        print!("{count:4}: {addr:#018x}  -  ");

        if let Some(ref elf) = elf
        && let Some(symtab) = elf.symbol_table()
        && let Some(sym) = symtab.find(|sym| sym.contains_addr(addr as _))
        && let Some(name) = sym.name()
        {
            println!("{:#}", rustc_demangle::demangle(name));
        } else {
            println!("<unknown>");
        }
    }
    println!("-----------------------");
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("kernel panic!\n{info}");

    static IN_PANIC: AtomicBool = AtomicBool::new(false);
    if !IN_PANIC.swap(true, Ordering::SeqCst) {
        trace_stack();
    } else {
        log::error!("double panic");
    }

    hcf();
}
