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

use self::cpu::ArchPcpu;
use crate::{pages_for, vm::VirtAddr};
use core::{
    str,
    sync::atomic::{compiler_fence, Ordering},
};
use spin::{mutex::SpinMutex, Mutex};

mod asm;
mod cpu;
mod msr;
pub mod trap;
pub mod vm;

pub use cpu::get_pcpu;

pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

#[allow(non_upper_case_globals)]
impl PrivilegeLevel {
    pub const Kernel: Self = Self::Ring0;
    pub const User: Self = Self::Ring3;
}

impl const From<PrivilegeLevel> for usize {
    fn from(pl: PrivilegeLevel) -> Self {
        pl as _
    }
}

impl const From<usize> for PrivilegeLevel {
    fn from(pl: usize) -> Self {
        match pl & 0x3 {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            3 => Self::Ring3,
            _ => unreachable!(),
        }
    }
}

/// Descriptor Privilege Level
pub type Dpl = PrivilegeLevel;

/// Requested Privilege Level
pub type Rpl = PrivilegeLevel;

static MEMORY_MAP: SpinMutex<limine::MemoryMapRequest> =
    SpinMutex::new(limine::MemoryMapRequest::new());

pub fn evil_putstr(s: &str) {
    unsafe {
        core::arch::asm!(
            "rep outsb",
            in("dx") 0x3f8,
            in("rsi") s.as_ptr(),
            in("rcx") s.len(),
            options(nostack, preserves_flags)
        );
    }
}

#[derive(Debug)]
pub struct Pcpu {
    arch: ArchPcpu,
}

impl core::ops::Deref for Pcpu {
    type Target = ArchPcpu;

    fn deref(&self) -> &Self::Target {
        &self.arch
    }
}

impl core::ops::DerefMut for Pcpu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.arch
    }
}

#[repr(C, align(0x1000))]
struct PageAlignedArray<const N: usize>([u8; N]);

// Rust wants all our stack!!
const KERNEL_STACK_SIZE: usize = 0x10_0000; // 1 MiB

#[naked]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    asm!(
        r#"

            mov     ebp, 0x2
            push    rbp
            popfq
            xor     ebp, ebp
            and     rsp, ~0xf
            call    {bolt_start}
        2:  cli
            hlt
            jmp     2b

        "#,
        bolt_start = sym bolt_start,
        options(noreturn)
    )
}

extern "C" fn bolt_start() {
    let mut pcpu_storage = PageAlignedArray([0; size_of!(Pcpu)]);
    let pcpu = pcpu_storage.0.as_mut_ptr().cast();
    unsafe { cpu::early_init(pcpu) };

    bolt::logger::init();

    static STACK_SIZE: limine::StackSizeRequest = limine::StackSizeRequest::new(KERNEL_STACK_SIZE);
    assert!(STACK_SIZE.has_response());

    let mut memmap = MEMORY_MAP.lock();
    let memmap = memmap.response_mut().expect("no memory map :^(");

    unsafe {
        static HHDM: limine::HhdmRequest = limine::HhdmRequest::new();
        let hhdm_base = HHDM.response().expect("no hddm :^(").base();
        bolt::vm::set_hhdm(VirtAddr::new(hhdm_base));
    }

    for entry in memmap.entries() {
        if entry.is_usable() {
            bolt::pmm::free_frames(entry.base().into(), pages_for!(entry.size()));
        }
    }

    static KERNEL_FILE: Mutex<limine::KernelFileRequest> =
        Mutex::new(limine::KernelFileRequest::new());
    let _kern_file = KERNEL_FILE.lock();
    let kern_file = _kern_file.response().unwrap().data().to_vec();
    bolt::panic::register_executable(kern_file).unwrap();

    compiler_fence(Ordering::SeqCst);

    cpu::init();
    trap::init();

    static BOOTLOADER_INFO_REQUEST: limine::BootloaderInfoRequest =
        limine::BootloaderInfoRequest::new();
    let bootloader_info = BOOTLOADER_INFO_REQUEST.response().unwrap();
    println!("{} {}", bootloader_info.brand(), bootloader_info.version());

    println!("dying on bsp");
    hcf();
}

/// Halt and catch fire
pub fn hcf() -> ! {
    loop {
        // SAFETY: Deadlocks are safe!!
        unsafe {
            asm!("cli; hlt; pause", options(nomem, nostack, preserves_flags));
        }
    }
}
