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

use super::{asm, msr, Pcpu, PrivilegeLevel, Rpl};
use crate::vm::VirtAddr;
use core::{
    fmt, ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

boltk_macros::bitstruct! {
    /// Processor Flags Register
    pub struct Rflags : usize {
        /// Carry Flag
        pub const CF    = 1, 0;
        /// Parity Flag
        pub const PF    = 1, 2;
        /// Auxiliary Carry Flag
        ///
        /// Indicates overflow out of bit 3, useful for BCD arithmetic.
        pub const AF    = 1, 4;
        /// Zero Flag
        ///
        /// The result of the last arithmetic operaton was zero.
        pub const ZF    = 1, 6;
        /// Sign Flag
        ///
        /// The sign bit was set in the result of the last arithmetic operation.
        pub const SF    = 1, 7;
        /// Trap Flag
        pub const TF    = 1, 8;
        /// Interrupt Flag
        pub const IF    = 1, 9;
        /// Direction Flag
        pub const DF    = 1, 10;
        /// Overflow Flag
        pub const OF    = 1, 11;
        /// I/O Permission Level
        pub const IOPL  = 2, 12 @ PrivilegeLevel;
        /// Nested Task
        pub const NT    = 1, 14;
        /// Resume Flag
        pub const RF    = 1, 16;
        ///
        pub const VM    = 1, 17;
        /// Access Control
        ///
        /// Setting this flag allows the supervisor to temporarily bypass the restrictions
        /// imposed by [SMAP](Cr4::SMAP).
        pub const AC    = 1, 18;
        /// Virtual Interrupt Flag
        pub const VIF   = 1, 19;
        /// Virtual Interrupt Pending
        pub const VIP   = 1, 20;
        /// Identification
        ///
        /// The ability of software to modify the value of this flag indicates processor
        /// support for the `cpuid` instruction.
        pub const ID    = 1, 21;
    }
}

pub const TRAP_FMASK: Rflags = Rflags::DF | Rflags::IF | Rflags::TF | Rflags::AC;
bolt::asm_export!(TRAP_FMASK = TRAP_FMASK.bits);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Selector(u16);

bolt::asm_export!(SEL_KCODE = Selector::KCODE.0);
bolt::asm_export!(SEL_KDATA = Selector::KDATA.0);
bolt::asm_export!(SEL_UCODE32 = Selector::UCODE32.0);
bolt::asm_export!(SEL_UCODE = Selector::UCODE.0);
bolt::asm_export!(SEL_UDATA = Selector::UDATA.0);

/// Global Descriptor Table
#[repr(C)]
#[derive(Debug)]
pub struct Gdt {
    null: u64,
    kernel_code: u64,
    kernel_data: u64,
    user_code32: u64,
    user_data32: u64,
    user_code: u64,
    user_data: u64,
    tss: u128,
}

impl Selector {
    pub const NULL: Selector = Self(0);
    pub const KCODE: Selector = Self(offset_of!(Gdt, kernel_code) as u16 | Rpl::Kernel as u16);
    pub const KDATA: Selector = Self::new(2, Rpl::Kernel);
    pub const UCODE32: Selector = Self::new(3, Rpl::Kernel);
    pub const UDATA32: Selector = Self::new(4, Rpl::Kernel);
    pub const UCODE: Selector = Self::new(5, Rpl::Kernel);
    pub const UDATA: Selector = Self::new(6, Rpl::Kernel);
    pub const TSS: Selector = Self::new(7, Rpl::Kernel);

    pub const fn new(index: usize, rpl: Rpl) -> Selector {
        debug_assert!(index <= 7);
        Self((index as u16) << 3 | rpl as u16)
    }

    pub const fn to_u16(self) -> u16 {
        self.0
    }
}

impl From<Selector> for u64 {
    fn from(sel: Selector) -> u64 {
        sel.0 as _
    }
}

impl Gdt {
    pub fn new(tss: *mut Tss) -> Gdt {
        Gdt {
            null: 0x0000000000000000,
            kernel_code: 0x0020980000000000,
            kernel_data: 0x0000920000000000,
            user_code32: 0x00cffa000000ffff,
            user_data32: 0x00cff2000000ffff,
            user_code: 0x0020f80000000000,
            user_data: 0x0000f20000000000,
            tss: {
                let tbase = unsafe { (*tss).base() as u128 };
                let limit = Tss::limit() as u128;

                0x0000890000000000
                    | ((tbase & 0xffffffffff000000) << 32)
                    | ((tbase & 0x0000000000ffffff) << 16)
                    | ((limit & 0x00000000000f0000) << 32)
                    | limit & 0x000000000000ffff
            },
        }
    }

    pub unsafe fn load(&self) {
        asm! {
            // Load the GDTR, which stores the base and limit of the GDT.
            // The instruction expects a memory operand pointing to a 16-bit limit followed
            // immediately by a 64-bit base address. This structure is created on the stack,
            // positioned such that both values are properly aligned.
            "
                sub     rsp, 16
                mov     qword ptr [rsp + 8], {gdt_base}
                mov      word ptr [rsp + 6], {gdt_limit}
                lgdt    [rsp + 6]
                add     rsp, 16
            ",
            // Now that the GDT is loaded, we can set up all the segment registers.
            "
                mov     edx, {sel_kcode}
                push    rdx
                lea     rdx, [rip + 1f]
                push    rdx
                retfq
            1:  mov     edx, {sel_kdata}
                mov     ss, dx
                mov     ds, dx
                mov     es, dx
                mov     fs, dx
                mov     gs, dx
            ",
            // Load the Task Register with the selector for the TSS.
            "
                mov     edx, {sel_tss}
                ltr     dx
            ",
            gdt_base  = in(reg) self,
            gdt_limit = const core::mem::size_of::<Gdt>() - 1,
            sel_kcode = const Selector::KCODE.to_u16(),
            sel_kdata = const Selector::KDATA.to_u16(),
            sel_tss   = const Selector::TSS.to_u16(),
            out("rdx") _,
            // options(noreturn)
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct Tss {
    // This field is *not* part of the TSS, but is included to make the rest of
    // the fields aligned, to keep the crab happy.
    // Don't forget to take this into account when getting the base/limit.
    pad0: u32,
    rsvd0: u32,
    rsp: [AtomicUsize; 3],
    rsvd1: u64,
    ist: [AtomicUsize; 7],
    rsvd2: [u8; 10],
    iopbm_base: u16,
    pad1: u32,
}

static_assert!(size_of!(Tss) == 104 + 8);

impl fmt::Debug for Tss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tss")
            .field("rsp", &self.rsp)
            .field("ist", &self.ist)
            .finish()
    }
}

impl Tss {
    pub const fn new() -> Tss {
        #[allow(clippy::declare_interior_mutable_const)]
        const NULL: AtomicUsize = AtomicUsize::new(0);

        Self {
            pad0: 0,
            rsvd0: 0,
            rsp: [NULL; 3],
            rsvd1: 0,
            ist: [NULL; 7],
            rsvd2: [0; 10],
            iopbm_base: 105,
            pad1: 0,
        }
    }

    fn base(&self) -> usize {
        ptr::addr_of!(self.rsvd0).addr()
    }

    const fn limit() -> usize {
        offset_of!(Self, pad1) - offset_of!(Self, pad0) - 1
    }
}

pub enum Ist {
    None = 0,
    Ist1 = 1,
    Ist2 = 2,
    Ist3 = 3,
    Ist4 = 4,
    Ist5 = 5,
    Ist6 = 6,
    Ist7 = 7,
}

pub unsafe fn early_init(pcpu: *mut Pcpu) {
    let apcpu = ptr::addr_of_mut!((*pcpu).arch);

    static NEXT_CPU_ID: AtomicUsize = AtomicUsize::new(0);
    (*apcpu).cpu_id = NEXT_CPU_ID.fetch_add(1, Ordering::SeqCst);

    (*apcpu).tss = Tss::new();
    let gdt = Gdt::new(ptr::addr_of_mut!((*apcpu).tss));
    println!("{:#x?}", gdt);
    (*apcpu).gdt = gdt;

    println!("Loading GDT.");
    (*apcpu).gdt.load();

    let gs_base = ptr::addr_of_mut!((*apcpu).gs_base);
    (*gs_base).pcpu_ptr = pcpu;

    msr::wrmsr(msr::IA32_GS_BASE, gs_base as _);
    msr::wrmsr(msr::IA32_KERNEL_GS_BASE, 0);
}

#[derive(Debug)]
pub struct ArchPcpu {
    cpu_id: usize,
    gdt: Gdt,
    tss: Tss,
    gs_base: CoreLocalSegment,
}

#[asm_export(prefix = "pcpu_")]
#[derive(Debug)]
pub struct CoreLocalSegment {
    pcpu_ptr: *const Pcpu,
    kstackp: VirtAddr,
    ustackp: VirtAddr,
}

impl ArchPcpu {
    pub fn set_interrupt_stack(&self, ist: Ist, stack_ptr: VirtAddr) -> VirtAddr {
        let prev = self.tss.ist[ist as usize - 1].swap(stack_ptr.to_usize(), Ordering::SeqCst);
        VirtAddr::new(prev)
    }

    pub fn cpu_id(&self) -> usize {
        self.cpu_id
    }
}

pub static SMAP_ENABLED: AtomicBool = AtomicBool::new(false);
pub static NX_ENABLED: AtomicBool = AtomicBool::new(false);

pub(super) fn init() {
    let cpuid = raw_cpuid::CpuId::new();

    let mut cr0 = Cr0::read();
    let mut cr4 = Cr4::read();
    let mut efer = Efer::read();

    let feats = cpuid.get_feature_info().unwrap();
    let ext_feats = cpuid
        .get_extended_processor_and_feature_identifiers()
        .unwrap();

    // x87 FPUm
    assert!(feats.has_fpu());
    cr0.remove(Cr0::EM);
    cr0.insert(Cr0::MP | Cr0::NE);
    unsafe { asm!("fninit", options(nomem, nostack)) };

    // Enable processor caches
    cr0.remove(Cr0::CD | Cr0::NW);

    // SSE
    assert!(feats.has_sse() && feats.has_sse2());
    assert!(feats.has_fxsave_fxstor());
    assert!(feats.has_clflush());
    cr4.insert(Cr4::OSFXSR | Cr4::OSXMMEXCPT);

    // Respect page write permissions in supervisor mode.
    cr0.insert(Cr0::WP);

    // Enable the `syscall` instruction
    assert!(ext_feats.has_syscall_sysret());
    efer.insert(Efer::SCE);

    // Machine Check Architecture
    if feats.has_mca() && feats.has_mce() {
        cr4 |= Cr4::MCE;
    }

    // Enable the no-execute bit in page table entries
    if ext_feats.has_execute_disable() {
        efer |= Efer::NXE;
        NX_ENABLED.store(true, Ordering::Relaxed);
    }

    if let Some(leaf7) = cpuid.get_extended_feature_info() {
        if leaf7.has_smap() {
            cr4 |= Cr4::SMAP;
            SMAP_ENABLED.store(true, Ordering::Relaxed);
        }
        cr4.set(Cr4::SMEP, leaf7.has_smep());
        cr4.set(Cr4::UMIP, leaf7.has_umip());
    }

    unsafe {
        cr0.write();
        cr4.write();
        efer.write();
    }
}

pub fn get_pcpu() -> &'static Pcpu {
    let this_pcpu: *const Pcpu;

    unsafe {
        asm!("mov {:r}, gs:[pcpu_pcpu_ptr]", out(reg) this_pcpu, options(nostack, preserves_flags));
        &*this_pcpu
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct Cr0 : usize {
        /// Protection Enable
        const PE = 1 << 0;
        /// Monitor Coprocessor
        const MP = 1 << 1;
        /// Emulation
        const EM = 1 << 2;
        /// Task Switched
        const TS = 1 << 3;
        /// Extension Type
        const ET = 1 << 4;
        /// Numeric Error
        const NE = 1 << 5;
        /// Write-Protect
        ///
        /// When this flag is set, the supervisor will be prohibited from writing to non-
        /// writable user pages. Normally the write permissions of user pages are ignored
        /// by the supervisor.
        const WP = 1 << 16;
        /// Alignment Mask
        const AM = 1 << 18;
        /// Not Write-Thru
        const NW = 1 << 29;
        /// Cache Disable
        const CD = 1 << 30;
        /// Paging
        const PG = 1 << 31;
    }
}

impl Cr0 {
    /// Read the current value of the CR0 register.
    pub fn read() -> Cr0 {
        // SAFETY: Architecturally reserved bits are not enumerated by this type,
        // but must have their values preserved when this `Cr3` is written back.
        unsafe {
            let bits: usize;

            asm!("mov {}, cr0", out(reg) bits, options(nomem, nostack, preserves_flags));

            Self::from_bits_unchecked(bits)
        }
    }

    /// Write this value to the CR0 register.
    ///
    /// # Safety
    ///
    /// Calling this method on a [`Cr0`] value which was not previously obtained by a
    /// call to [`read()`] may result in unintended side effects, as architecturally
    /// reserved bits (or those not yet enumated by this type) must have their values
    /// preserved by writes.
    ///
    /// Additionally, certain combinations of defined bits may cause unintended or
    /// undefined behavior. Consult the relevant processor manual for more information.
    pub unsafe fn write(self) {
        asm!("mov cr0, {}", in(reg) self.bits(), options(nomem, nostack, preserves_flags));
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct Cr4 : usize {
        /// Virtual-8086 Mode Extensions
        const VME        = 1 << 0;
        /// Protected-Mode Virtual Interrupts
        const PVI        = 1 << 1;
        /// Time Stamp Disable
        ///
        /// When this flag is set, execution of the `rdtsc` (and `rdtscp`, if supported) is
        /// restricted to [ring 0](PrivilegeLevel::Kernel).
        const TSD        = 1 << 2;
        /// Debugging Extensions
        const DE         = 1 << 3;
        /// Page Size Extensions
        const PSE        = 1 << 4;
        /// Physical Address Extension
        const PAE        = 1 << 5;
        /// Machine-Check Enable
        const MCE        = 1 << 6;
        /// Page Global Enable
        const PGE        = 1 << 7;
        /// Performance-Monitoring Counter Enable
        ///
        /// When this flag is set, execution of the `rdpmc` instruction is allowed for all
        /// privilege levels. Execution is restricted to [ring 0](PrivilegeLevel::Kernel)
        /// when clear.
        const PCE        = 1 << 8;
        /// Operating System Support for FXSAVE and FXRSTOR instructions
        const OSFXSR     = 1 << 9;
        /// Operating System Support for Unmasked SIMD Floating-Point Exceptions
        const OSXMMEXCPT = 1 << 10;
        /// User-Mode Instruction Prevention
        ///
        /// When this flag is set, execution of the following instructions is restricted to
        /// [ring 0](PrivilegeLevel::Kernel):
        ///
        /// - `sgdt`
        /// - `sidt`
        /// - `sldt`
        /// - `smsw`
        /// - `str`
        ///
        /// Attempts to execute these instructions from other privilege levels will cause
        /// a #GP exception.
        const UMIP       = 1 << 11;
        /// 57-bit linear addresses (5-level paging)
        const LA57       = 1 << 12;
        /// VMX
        const VMXE       = 1 << 13;
        /// SMX
        const SMXE       = 1 << 14;
        /// FSGSBASE
        ///
        /// When this flag is set, the following instructions are enabled:
        ///
        /// - `rdfsbase`
        /// - `wrfsbase`
        /// - `rdgsbase`
        /// - `wrgsbase`
        ///
        /// These instructions allow faster access to the base addresses of the FS and
        /// GS segments, particularly writes as the `wrmsr` instruction is serializing.
        const FSGSBASE   = 1 << 16;
        /// Process-Context Identifiers
        const PCIDE      = 1 << 17;
        /// XSAVE and Processor Extended States
        const OSXSAVE    = 1 << 18;
        /// Key Locker Enable
        const KL         = 1 << 19;
        /// Supervisor-Mode Execution Prevention
        ///
        /// When this flag is set, the supervisor is prohibited from executing code
        /// from pages mapped as user-accessible.
        const SMEP       = 1 << 20;
        /// Supervisor-Mode Access Prevention
        ///
        /// When this flag is set, the supervisor is prohibited from reading from pages
        /// mapped as user-accessible. This behavior can temporarily be disabled by the
        /// supervisor by setting [`Rflags::AC`]. This feature has no affect on the
        /// restrictions imposed by [`Cr0::WP`].
        const SMAP       = 1 << 21;
        /// User-Mode Protection Keys
        const PKE        = 1 << 22;
        /// Control-Flow Enforcement Technology
        const CET        = 1 << 23;
        /// Supervisor-Mode Protection Keys
        const PKS        = 1 << 24;
    }
}

impl Cr4 {
    /// Read the current value of the CR4 register.
    pub fn read() -> Cr4 {
        // SAFETY: Architecturally reserved bits are not enumerated by this type,
        // but must have their values preserved when this `Cr4` is written back.
        unsafe {
            let bits: usize;

            asm!("mov {}, cr4", out(reg) bits, options(nomem, nostack, preserves_flags));

            Self::from_bits_unchecked(bits)
        }
    }

    /// Write this value to the CR4 register.
    ///
    /// # Safety
    ///
    /// Calling this method on a [`Cr4`] value which was not previously obtained by a
    /// call to [`read()`] may result in unintended side effects, as architecturally
    /// reserved bits (or those not yet enumated by this type) must have their values
    /// preserved by writes.
    ///
    /// Additionally, certain combinations of defined bits may cause unintended or
    /// undefined behavior. Consult the relevant processor manual for more information.
    pub unsafe fn write(self) {
        asm!("mov cr4, {}", in(reg) self.bits(), options(nomem, nostack, preserves_flags));
    }
}

bitflags::bitflags! {
    /// Extended Features Enable Register
    pub struct Efer : u64 {
        /// SYSCALL Enable
        ///
        /// Setting this flag enables the `syscall` and `sysret` instructions.
        const SCE = 1 << 0;
        /// Long Mode Enable
        ///
        /// Setting this flag enables IA-32e Mode (Long Mode) operation.
        const LME = 1 << 8;
        /// Long Mode Active
        ///
        /// This flag is read-only and is set by the processor when executing in
        /// IA-32e Mode.
        const LMA = 1 << 10;
        /// Execute-Disable-Bit Enable
        ///
        /// Setting this flag enables the Execute-Disable (NX) bit in page table entries.
        /// By default all pages are executable and the NX bit is reserved (must be 0).
        const NXE = 1 << 11;
    }
}

impl Efer {
    /// Read the current value of the EFER MSR.
    pub fn read() -> Efer {
        // SAFETY: Architecturally reserved bits are not enumerated by this type,
        // but must have their values preserved when this `Efer` is written back.
        unsafe { Self::from_bits_unchecked(msr::rdmsr(msr::IA32_EFER)) }
    }

    /// Write this value to the EFER MSR.
    ///
    /// # Safety
    ///
    /// Calling this method on an [`Efer`] value which was not previously obtained by a
    /// call to [`read()`] may result in unintended side effects, as architecturally
    /// reserved bits (or those not yet enumated by this type) must have their values
    /// preserved by writes.
    ///
    /// Additionally, certain combinations of defined bits may cause unintended or
    /// undefined behavior. Consult the relevant processor manual for more information.
    pub unsafe fn write(self) {
        msr::wrmsr(msr::IA32_EFER, self.bits() as _);
    }
}
