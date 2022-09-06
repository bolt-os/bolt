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

use crate::arch::{asm, cpu::TRAP_FMASK, msr};

use super::cpu::Rflags;

use {
    bolt::arch::{
        cpu::{get_pcpu, Ist, Selector},
        vm::with_userspace_access,
        Dpl,
    },
    bolt::{pmm, run_once, vm::PAGE_SIZE},
    core::ptr,
    spin::mutex::SpinMutex,
};

#[repr(C)]
#[derive(Debug)]
#[asm_export(prefix = "tf_")]
pub struct TrapFrame {
    rdi: usize,
    rsi: usize,
    rdx: usize,
    r10: usize,
    r8: usize,
    r9: usize,
    rax: usize,
    rbx: usize,
    rcx: usize,
    rbp: usize,
    r11: usize,
    r12: usize,
    r13: usize,
    r14: usize,
    r15: usize,
    info: usize,
    error: usize,
    rip: usize,
    cs: usize,
    rflags: usize,
    rsp: usize,
    ss: usize,
}

pub struct Exception {
    pub name: &'static str,
}

impl bolt::trap::TrapFrameApi for TrapFrame {
    fn is_exception(&self) -> bool {
        (self.info & 0xff) < 32
    }

    fn exception(&self) -> Option<&'static crate::trap::Exception> {
        None
    }
}

#[repr(transparent)]
struct InterruptDescriptorTable {
    descriptors: [u128; 256],
}

impl InterruptDescriptorTable {
    pub const fn new() -> InterruptDescriptorTable {
        Self {
            descriptors: [0; 256],
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Vector(u8);

impl Vector {
    #![allow(dead_code)]

    pub const DIVIDE_ERROR: Vector = Vector(0);
    pub const DEBUG: Vector = Vector(1);
    pub const NON_MASKABLE_INTERRUPT: Vector = Vector(2);
    pub const BREAKPOINT: Vector = Vector(3);
    pub const SIGNED_OVERFLOW: Vector = Vector(4);
    pub const BOUND_RANGE_EXCEEDED: Vector = Vector(5);
    pub const INVALID_OPCODE: Vector = Vector(6);
    pub const DEVICE_NOT_AVAILABLE: Vector = Vector(7);
    pub const DOUBLE_FAULT: Vector = Vector(8);
    pub const INVALID_TSS: Vector = Vector(10);
    pub const SEGMENT_NOT_PRESENT: Vector = Vector(11);
    pub const STACK_FAULT: Vector = Vector(12);
    pub const GENERAL_PROTECTION_FAULT: Vector = Vector(13);
    pub const PAGE_FAULT: Vector = Vector(14);
    pub const FPU_ERROR: Vector = Vector(16);
    pub const ALIGNMENT_CHECK: Vector = Vector(17);
    pub const MACHINE_CHECK: Vector = Vector(18);
    pub const SIMD_FPU_ERROR: Vector = Vector(19);
    pub const VIRTUALIZATION: Vector = Vector(20);
    pub const CONTROL_PROTECTION: Vector = Vector(21);

    pub const SYSCALL: Vector = Vector(0x80);
}

impl Vector {
    pub const fn to_usize(self) -> usize {
        self.0 as _
    }

    pub const fn ist(self) -> Ist {
        match self {
            Vector::DEBUG => Ist::Ist1,
            Vector::DOUBLE_FAULT => Ist::Ist2,
            Vector::NON_MASKABLE_INTERRUPT => Ist::Ist3,
            Vector::MACHINE_CHECK => Ist::Ist4,
            _ => Ist::None,
        }
    }
}

pub fn vectors() -> impl Iterator<Item = Vector> {
    (0..=255).map(Vector)
}

pub fn disable() {
    unsafe { super::asm::cli() };
}

pub fn init() {
    static IDT: SpinMutex<InterruptDescriptorTable> =
        SpinMutex::new(InterruptDescriptorTable::new());

    /*
     * The IDT needs to be set up with the entry points defined in trap.asm.
     * Since the IDT is shared among all cores, this only needs to be done once.
     */
    run_once! {
        extern "sysv64" {
            static trap_stubs: [usize; 256];
        }

        let mut descriptors = [0; 256];

        for v in 0..256usize {
            let offset = unsafe { trap_stubs[v] } as u128;
            let dpl = if v == Vector::SYSCALL.to_usize() {
                (Dpl::User as u128) << 46
            } else {
                (Dpl::Kernel as u128) << 46
            };

            descriptors[v] = (0x8e << 40) | dpl
                | ((Vector(v as u8).ist() as u128) << 32)
                | ((Selector::KCODE.to_u16() as u128) << 16)
                | ((offset & 0xffffffff00000000) << 32)
                | ((offset & 0x00000000ffff0000) << 32)
                | (offset & 0x000000000000ffff);
        }

        IDT.lock().descriptors.copy_from_slice(&descriptors);
    }

    /*
     * An interrupt stack needs to be allocated for each in-use IST entry.
     */
    let pcpu = get_pcpu();
    let ists = vectors().filter_map(|v| match v.ist() {
        Ist::None => None,
        ist => Some(ist),
    });
    for ist in ists {
        let stack_ptr = pmm::alloc_frames(4).unwrap().to_virtual() + 4 * PAGE_SIZE;
        let prev = pcpu.set_interrupt_stack(ist, stack_ptr);
        assert!(prev.is_null());
    }

    /*
     * Load the IDTR to point to the IDT.
     */
    unsafe {
        asm!(
            r#"
                sub     rsp, 16
                mov     word ptr  [rsp + 6], {}
                mov     qword ptr [rsp + 8], {}
                lidt    [rsp + 6]
                add     rsp, 16
            "#,
            const size_of!(InterruptDescriptorTable) - 1,
            in(reg) IDT.as_mut_ptr(),
            options(nomem)
        );
    }

    /*
     * Set up the fast system call mechanisms.
     */

    // SYSENTER
    unsafe {
        msr::wrmsr(msr::IA32_SYSENTER_CS, Selector::KCODE.to_u16() as _);
        msr::wrmsr(msr::IA32_SYSENTER_EIP, trap_sysenter as usize as _);
    }

    // SYSCALL
    let star = u64::from(Selector::UCODE32) << 48 | u64::from(Selector::KCODE) << 32;
    unsafe {
        msr::wrmsr(msr::IA32_STAR, star);
        msr::wrmsr(msr::IA32_LSTAR, trap_syscall as usize as _);
        msr::wrmsr(msr::IA32_FMASK, TRAP_FMASK.bits() as _);
    }

    unsafe { (0xcafebabecafebabe as *mut u8).read_volatile() };
}

pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_if = unsafe { asm::push_cli() };
    let result = f();
    if prev_if {
        unsafe { asm::sti() };
    }
    result
}

const T_SYSENTER: usize = 256;
const T_SYSENTER_UD: usize = 257;
const T_SYSCALL: usize = 258;

/// Catch the #UD exception generated by AMD processors when `sysenter` is executed
fn catch_sysenter(tf: &mut TrapFrame) {
    const SYSENTER_OPCODE: u16 = 0x340f;

    if tf.rip == 0 {
        return;
    }

    let fault_opcode =
        with_userspace_access(|| unsafe { ptr::read_unaligned(tf.rip as *const u16) });

    /*
     * System calls with `sysenter` normally pass the return RIP and RSP in the RCX and R11
     * registers, respectively, which are then pushed by the handler. Since we're entering
     * through the IDT mechanism instead, we need to update the values pushed by the CPU,
     * and also switch the trap info value so this will be handled as a system call.
     */
    if fault_opcode == SYSENTER_OPCODE {
        tf.rip = tf.rcx;
        tf.rsp = tf.r11;
        tf.info = T_SYSENTER_UD;
    }
}

macro_rules! push_registers {
    () => {
        r#"
            sub     rsp, 8 * 15
            mov     [rsp + 8 *  0], rdi
            mov     [rsp + 8 *  1], rsi
            mov     [rsp + 8 *  2], rdx
            mov     [rsp + 8 *  3], r10
            mov     [rsp + 8 *  4], r8
            mov     [rsp + 8 *  5], r9
            mov     [rsp + 8 *  6], rax
            mov     [rsp + 8 *  7], rbx
            mov     [rsp + 8 *  8], rcx
            mov     [rsp + 8 *  9], rbp
            mov     [rsp + 8 * 10], r11
            mov     [rsp + 8 * 11], r12
            mov     [rsp + 8 * 12], r13
            mov     [rsp + 8 * 13], r14
            mov     [rsp + 8 * 14], r15
        "#
    };
}
macro_rules! pop_registers {
    () => {
        r#"
            mov     rdi, [rsp + 8 *  0]
            mov     rsi, [rsp + 8 *  1]
            mov     rdx, [rsp + 8 *  2]
            mov     r10, [rsp + 8 *  3]
            mov     r8,  [rsp + 8 *  4]
            mov     r9,  [rsp + 8 *  5]
            mov     rax, [rsp + 8 *  6]
            mov     rbx, [rsp + 8 *  7]
            mov     rcx, [rsp + 8 *  8]
            mov     rbp, [rsp + 8 *  9]
            mov     r11, [rsp + 8 * 10]
            mov     r12, [rsp + 8 * 11]
            mov     r13, [rsp + 8 * 12]
            mov     r14, [rsp + 8 * 13]
            mov     r15, [rsp + 8 * 14]
            add     rsp, 8 * 15
        "#
    };
}

#[naked]
#[no_mangle]
unsafe extern "sysv64" fn trap_common() {
    asm! {
        push_registers!(),
        r#"
            mov     r15, rsp

            mov     rax, [r15 + tf_rip]
            push    rax
            push    rbp
            mov     rbp, rsp
            test    dword ptr [r15 + tf_cs], 0x3
            jz      3f
            xor     ebp, ebp
            swapgs
            lfence
        3:

            cld
            and     rsp, ~0xf
            mov     rdi, r15
            cmp     byte ptr [r15 + tf_info], 6
            jne     3f
            call    {catch_sysenter}
            mov     rdi, r15
        3:  call    {trap_dispatch}
            test    dword ptr [r15 + tf_cs], 0x3
            jz      3f
            swapgs
            lfence
        3:  mov     rsp, r15
        "#,
        pop_registers!(),
        r#"
            add     rsp, 0x10
            iretq
        "#,
        catch_sysenter = sym catch_sysenter,
        trap_dispatch  = sym bolt::trap::dispatch,
        options(noreturn),
    }
}

#[naked]
unsafe extern "sysv64" fn trap_sysenter() {
    asm! {
        r#"
            swapgs
            push    {SEL_UDATA}
            push    r11
            pushfq
            mov     r11, [rsp]
            and     dword ptr [rsp], {TRAP_FMASK}
            popfq
            push    r11
            push    {SEL_UCODE}
            push    rcx
            push    0
            push    {T_SYSENTER}
        "#,
        push_registers!(),
        r#"
            mov     r15, rsp
            xor     ebp, ebp
            and     rsp, ~0xf
            mov     rdi, r15
            call    {trap_dispatch}
            mov     rsp, r15
        "#,
        pop_registers!(),
        r#"
            mov     rdx, [rsp + 0x00]
            mov     rcx, [rsp + 0x18]
            and     dword ptr [rsp + 0x10], {EXIT_FMASK}
            add     rsp, 0x10
            popfq
            swapgs
            sti
            sysexitq
        "#,
        EXIT_FMASK = const !(Rflags::TF | Rflags::IF).bits(),
        SEL_UCODE  = const Selector::UCODE.to_u16(),
        SEL_UDATA  = const Selector::UDATA.to_u16(),
        T_SYSENTER = const T_SYSENTER,
        TRAP_FMASK = const TRAP_FMASK.bits(),
        trap_dispatch  = sym bolt::trap::dispatch,
        options(noreturn),
    }
}

#[naked]
unsafe extern "sysv64" fn trap_syscall() {
    asm! {
        r#"
            /*
             * Swap in the kernel's GS base and switch to the
             * kernel's stack.
             */
            swapgs
            mov     gs:[pcpu_ustackp], rsp
            mov     rsp, gs:[pcpu_kstackp]

            /*
             * The return RIP and RFLAGS have been saved in RCX
             * and R11, respectively. We want to build the same
             * stack frame that is built when an interrupt occurs
             * through the IDT mechanism.
             */
            push    {SEL_UDATA}
            push    qword ptr gs:[pcpu_ustackp]
            push    r11
            push    {SEL_UCODE}
            push    rcx
            push    0
            push    {T_SYSCALL}
        "#,
        push_registers!(),
        r#"
            /*
             * Save the stack pointer (pointing to the `TrapFrame`) in RBP,
             * then align the stack for Rust. RFLAGS.DF was already cleared
             * when SYSCALL was executed.
             */
            mov     r15, rsp
            xor     ebp, ebp
            and     rsp, ~0xf
            mov     rdi, r15
            call    {trap_dispatch}
            mov     rsp, r15
        "#,
        pop_registers!(),
        r#"
            /*
             * Switch back to the user's stack and swap GS base.
             * We don't need to clear the stack. RCX and R11 were not modified before
             * `push_registers!()`, so they'll have been properly restored.
             */
            mov     rsp, gs:[pcpu_ustackp]
            swapgs
            sysretq
        "#,
        T_SYSCALL = const T_SYSCALL,
        SEL_UCODE = const Selector::UCODE.to_u16(),
        SEL_UDATA = const Selector::UDATA.to_u16(),
        trap_dispatch  = sym bolt::trap::dispatch,
        options(noreturn),
    }
}
