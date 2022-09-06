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

use bolt::trap::{Exception, Vector};
use core::{fmt, ptr};

static EXCEPTIONS: &[Exception] = &[
    Exception {
        vector: 0,
        name: "instruction address misaligned",
    },
    Exception {
        vector: 1,
        name: "instruction address fault",
    },
    Exception {
        vector: 2,
        name: "illegal instruction",
    },
    Exception {
        vector: 3,
        name: "breakpoint",
    },
    Exception {
        vector: 4,
        name: "load address misaligned",
    },
    Exception {
        vector: 5,
        name: "load access fault",
    },
    Exception {
        vector: 6,
        name: "store/amo address misaligned",
    },
    Exception {
        vector: 7,
        name: "store/amo access fault",
    },
    Exception {
        vector: 8,
        name: "environment call from u-mode",
    },
    Exception {
        vector: 9,
        name: "environment call from s-mode",
    },
    Exception {
        vector: 10,
        name: "reserved",
    },
    Exception {
        vector: 11,
        name: "reserved",
    },
    Exception {
        vector: 12,
        name: "instruction page fault",
    },
    Exception {
        vector: 13,
        name: "load page fault",
    },
    Exception {
        vector: 14,
        name: "reserved",
    },
    Exception {
        vector: 15,
        name: "store/amo page fault",
    },
];

#[repr(C)]
// #[derive(Debug)]
pub struct TrapFrame {
    cause: usize,
    epc: usize,
    val: usize,
    sstatus: super::cpu::Sstatus,
    gpr: [usize; 32],
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("gpr", &self.gpr)
            .field("scause", &self.cause)
            .field("stval", &ptr::invalid::<()>(self.val))
            .field("sepc", &ptr::invalid::<()>(self.epc))
            .field("sstatus", &ptr::invalid::<()>(self.sstatus.bits()))
            .finish()
    }
}

impl bolt::trap::TrapFrameApi for TrapFrame {
    fn is_exception(&self) -> bool {
        self.cause & 1 << (usize::BITS - 1) == 0
    }

    fn exception(&self) -> Option<&'static Exception> {
        self.is_exception()
            .then(|| EXCEPTIONS.get(self.cause))
            .flatten()
    }
}

include_asm!("trap.s");

pub fn init() {
    extern "C" {
        fn trap_entry();
    }
    unsafe {
        asm!("csrw stvec, {}", in(reg) trap_entry, options(nostack));
    }

    unsafe { (0xcafebabecafebabe as *mut u8).read_volatile() };
}

#[export_name = "rust_trap_entry"]
extern "C" fn dispatch(tf: &mut TrapFrame) {
    bolt::trap::dispatch(tf);
}

pub fn disable() {
    unsafe { asm!("csrci sstatus, 0x2") };
}

pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_sstatus: usize;

    unsafe {
        asm!(
            "csrrci {0}, sstatus, 0x2",
            out(reg) prev_sstatus,
            options(nomem, nostack, preserves_flags),
        );
    }

    let result = f();

    if prev_sstatus & 0x2 != 0 {
        unsafe {
            asm!(
                "csrsi sstatus, 0x2",
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    result
}

pub fn with_userspace_access<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    without_interrupts(|| {
        let prev_sstatus: usize;

        unsafe {
            asm!(
                "csrrc {0}, sstatus, {0}",
                inout(reg) 1usize << 18 => prev_sstatus,
                options(nomem, nostack, preserves_flags),
            );
        }

        let result = f();

        if prev_sstatus & 1 << 18 != 0 {
            unsafe {
                asm!(
                    "csrs sstatus, {}",
                    in(reg) 1 << 18,
                    options(nomem, nostack, preserves_flags)
                );
            }
        }

        result
    })
}
