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

use core::sync::atomic::{AtomicBool, Ordering};

use spin::RwLock;

pub fn hcf() -> ! {
    bolt::trap::disable();
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("kernel panic!");
    println!("{info}");

    static PANICKING: AtomicBool = AtomicBool::new(false);
    if !PANICKING.swap(true, Ordering::SeqCst) {
        trace_stack();
    }

    println!("dead");
    hcf();
}

trait Foo: Sync {}
impl Foo for elf::Elf<'static> {}

static KERNEL_ELF: RwLock<Option<Vec<u8>>> = RwLock::new(None);

pub fn register_executable(file: Vec<u8>) -> Result<(), ()> {
    let mut kern_elf = KERNEL_ELF.write();

    if kern_elf.is_some() {
        return Err(());
    }

    *kern_elf = Some(file);
    Ok(())
}

#[inline(never)]
fn trace_stack() {
    use unwinding::abi::*;

    println!("----- STACK TRACE -----");
    let elf_guard = KERNEL_ELF.read();
    let elf = elf_guard.as_ref().and_then(|data| elf::Elf::new(data).ok());
    let mut count = 0usize;
    uw::backtrace(&mut count, |ctx, count| {
        let ip = _Unwind_GetIP(ctx);
//
        print!("{count:4}: {ip:#018x}  -  ");
//
        if let Some(ref elf) = elf
            && let Some(symtab) = elf.symbol_table()
            && let Some(sym) = symtab.find(|sym| sym.contains_addr(ip as _))
            && let Some(name) = sym.name()
        {
            // println!("{}", rustc_demangle::demangle(name));
            println!("{name}");
        } else {
            println!("<unknown>");
        }
//
        *count += 1;
        UnwindReasonCode::NO_REASON
    });
    println!("-----------------------");
}

mod uw {
    use core::ffi::c_void;

    use unwinding::abi::{UnwindContext, UnwindReasonCode, _Unwind_Backtrace};

    struct TraceData<'a, T> {
        data: &'a mut T,
        f: &'a mut dyn FnMut(&mut UnwindContext<'_>, &mut T) -> UnwindReasonCode,
    }

    pub fn backtrace<F, T>(data: &mut T, mut f: F) -> UnwindReasonCode
    where
        F: FnMut(&mut UnwindContext<'_>, &mut T) -> UnwindReasonCode,
    {
        extern "C" fn backtrace_callback<T>(
            ctx: &mut UnwindContext<'_>,
            data: *mut c_void,
        ) -> UnwindReasonCode {
            let data = unsafe { &mut *data.cast::<TraceData<T>>() };
            (data.f)(ctx, data.data)
        }

        let mut data = TraceData { data, f: &mut f };
        let data = &mut data as *mut _ as *mut c_void;

        _Unwind_Backtrace(backtrace_callback::<T>, data)
    }
}
