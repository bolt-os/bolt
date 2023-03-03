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

use crate::{
    arch::{x86_64::msr, ThisArch},
    cpu::{self, Cpu},
    sync::lazy::Lazy,
};
use core::{
    mem::{self, size_of},
    ptr::addr_of_mut,
};
use cpu_features::{CpuFeatures, CpuInfo};
use memoffset::offset_of;

impl cpu::ArchCpu for ThisArch {
    type Data = CpuData;

    #[inline(always)]
    fn get_current_cpu() -> *mut Cpu {
        let this_cpu;

        unsafe {
            asm!(
                "mov {:r}, gs:[{}]",
                out(reg) this_cpu,
                const offset_of!(Cpu, md_data) + offset_of!(CpuData, this_cpu),
                options(nostack, preserves_flags),
            );
        }

        this_cpu
    }
}

pub struct CpuData {
    this_cpu: *mut Cpu,
    gdt: Gdt,
    tss: Tss,
    cpu_info: CpuInfo,
}

#[repr(C)]
struct Gdt {
    null: usize,
    kernel_code: usize,
    kernel_data: usize,
    user_code_32: usize,
    user_data_32: usize,
    user_code: usize,
    user_data: usize,
    tss: usize,
    tss_hi: usize,
}

#[repr(C, packed)]
struct Tss {
    reserved0: u32,
    pub(super) privileged_stack_table: [usize; 3],
    pub(super) interrupt_stack_table: [usize; 8],
    reserved1: [u16; 5],
    io_map_base: u16,
}

pub static CPU_FEATURES: Lazy<CpuFeatures> = Lazy::new(|| unimplemented!());

pub unsafe fn early_init(cpu: *mut Cpu) {
    let mdcpu = addr_of_mut!((*cpu).md_data);

    // Set up the self-reference.
    (*mdcpu).this_cpu = cpu;

    // Initialize the TSS. Set `io_map_base` to the size of the TSS to disable
    // the I/O Permission Bitmap.
    let tss = addr_of_mut!((*mdcpu).tss);
    tss.write(Tss {
        io_map_base: size_of::<Tss>() as u16,
        ..mem::zeroed()
    });

    let tss_base = tss as usize;
    let tss_limit = size_of::<Tss>() - 1;
    let tss_desc_hi = tss_base >> 32;
    let tss_desc_lo = 0x89 << 40
        | (tss_base & 0xff000000) << 32
        | (tss_base & 0x00ffffff) << 16
        | (tss_limit & 0xf0000) << 32
        | (tss_limit & 0x0ffff);

    // Initialize the GDT.
    // For more information about these values see the Intel Software Developer's Manual.
    let gdt = addr_of_mut!((*mdcpu).gdt);
    gdt.write(Gdt {
        null: 0x0000000000000000,
        kernel_code: 0x00209a0000000000,
        kernel_data: 0x0000920000000000,
        user_code_32: 0x00cffa000000ffff,
        user_data_32: 0x00cff2000000ffff,
        user_code: 0x0060fa0000000000,
        user_data: 0x0000f20000000000,
        tss: tss_desc_lo,
        tss_hi: tss_desc_hi,
    });

    let cpu_info = cpu_features::init();
    Lazy::initialize_with(&CPU_FEATURES, cpu_info.features.clone());
    addr_of_mut!((*mdcpu).cpu_info).write(cpu_info);

    asm!(
        // Load the Global Descriptor Table Register.
        //
        // The `lgdt` instruction expects a memory operand pointing to a 16-bit
        // value containing the limit of the GDT (the offset of the last byte,
        // or `size_of::<Gdt>() - 1`) followed by a 64-bit value containing the
        // base address of the table.
        //
        // This structure is created on the stack, at an offset which both values
        // are properly aligned.
        "
            sub     rsp, 16
            mov     word ptr [rsp + 6], {gdt_limit}
            mov     [rsp + 8], {0:r}
            lgdt    [rsp + 6]
            add     rsp, 16
        ",

        // Load the code segment register.
        //
        // We cannot use a simple far jump to set CS in Long Mode.
        // Instead, we create a far call frame on the stack and execute a far return
        // to the following instruction.
        "
            push    {SEL_KCODE}  // kernel code selector
            lea     {0:r}, [rip + 1f]
            push    {0:r}
            retfq
        1:
        ",

        // Load the data segment registers.
        //
        // Note that loading FS and GS in this way overwrites the base addresses, so
        // must be done *before* we set the full, 64-bit `Cpu` pointer via the MSR.
        "
            mov     {0:e}, {SEL_KDATA}  // kernel data selector
            mov     ds, {0:x}
            mov     es, {0:x}
            mov     fs, {0:x}
            mov     gs, {0:x}
            mov     ss, {0:x}
        ",

        // Load the Task Register.
        //
        // The Task Register points to the active TSS. It contains the GDT selector
        // for the TSS system segment.
        "
            mov     {0:e}, {SEL_TSS}
            ltr     {0:x}
        ",

        inlateout(reg) gdt => _,
        gdt_limit = const size_of::<Gdt>() - 1,
        SEL_KCODE = const offset_of!(Gdt, kernel_code),
        SEL_KDATA = const offset_of!(Gdt, kernel_data),
        SEL_TSS   = const offset_of!(Gdt, tss),
    );

    msr::wrmsr(msr::IA32_GS_BASE, cpu as u64);
    msr::wrmsr(msr::IA32_KERNEL_GS_BASE, 0);
}
