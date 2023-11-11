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

//! Hardware Address Translation

use core::{
    cmp, fmt,
    ops::{self, Index, IndexMut, Range},
};

use cpu_features::CpuFeat;
use x86_64::{control::Cr4, msr::Efer};

use super::cpu::{self, CPU_FEATURES};
use crate::{
    arch::ThisArch,
    sync::{lazy::Lazy, mutex::MutexKind, Mutex},
    util::{bootstrap_cell::BootstrapCell, pow2, size_of},
    vm::{self, page::PMAP_QUEUE, PhysAddr, Prot, VirtAddr, PAGE_SIZE},
};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    AlreadyMapped(PhysAddr),
    HugePage(PageSize, PhysAddr),
}

impl VirtAddr {
    const fn index_for(self, level: u32) -> usize {
        self.0 >> (12 + 9 * level) & 0x1ff
    }

    fn is_canonical(self) -> bool {
        !MMU_INFO.noncanonical_hole.contains(&self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSize {
    Size4KiB = 0,
    Size2MiB = 1,
    Size1GiB = 2,
}

/// Hardware Address Translation Context
///
/// # Kernel HAT
pub struct Hat {
    /// Top-level page table
    top_level:    &'static vm::Page,
    /// Tracks the number of pages of each size mapped into the address space
    mapped_pages: [usize; MAX_PAGE_LEVEL + 1],
    pcid:         usize,
    cr3:          usize,
}

unsafe impl Send for Hat {}

impl Drop for Hat {
    fn drop(&mut self) {
        log::warn!("TODO: Hat::drop()");
    }
}

impl Hat {
    pub fn new(asid: usize) -> Mutex<Hat> {
        // Allocate the top-level table and initialize the global entries.
        let page = vm::Page::alloc(&mut PMAP_QUEUE.lock()).unwrap();
        unsafe {
            page.addr
                .to_virt()
                .as_mut_ptr::<Pte>()
                .copy_from(INITIAL_PTES.as_ptr(), PTES_PER_TABLE);
        }
        let pcid = asid & 0xfff;
        let cr3 = if MMU_INFO.pcide {
            page.addr.0 | pcid
        } else {
            page.addr.0
        };
        Mutex::new(MutexKind::Adaptive, Hat {
            cr3,
            pcid,
            top_level: page,
            mapped_pages: [0; MAX_PAGE_LEVEL + 1],
        })
    }

    pub fn unmap_pages(&mut self, virt: VirtAddr, size: usize, page_size: PageSize) {
        let map_level = if page_size == PageSize::Size1GiB && !MMU_INFO.gigapages {
            PageSize::Size2MiB as usize
        } else {
            page_size as usize
        };
        let page_size = MMU_INFO.page_size[map_level];

        debug_assert!(virt.is_aligned(page_size));
        debug_assert!(size & (page_size - 1) == 0);

        let mut unmap_virt = virt;
        let mut num_pages = size / page_size;

        while num_pages > 0 {
            let mut table = self.top_level.addr.to_virt().as_mut_ptr::<Pte>();
            let mut level = MMU_INFO.max_level;

            loop {
                let index = unmap_virt.index_for(level);
                let entry_ptr = unsafe { table.add(index) };
                let entry = unsafe { entry_ptr.read_volatile() };

                // Handle parent entries.
                if level != map_level as u32 {
                    assert!(entry.present());
                    assert!(!entry.huge());
                    table = entry.addr().to_virt().as_mut_ptr();
                    level -= 1;
                    continue;
                }

                let unmap_pages = cmp::min(num_pages, PTES_PER_TABLE - index);
                for i in 0..unmap_pages {
                    let entry_ptr = unsafe { entry_ptr.add(i) };
                    let entry = unsafe { entry_ptr.read_volatile() };
                    assert!(entry.present());
                    unsafe { entry_ptr.write_volatile(Pte::NULL) };
                    unmap_virt += page_size;
                }

                num_pages -= unmap_pages;
                break;
            }
        }
    }

    /// Insert a translation into the address space
    ///
    /// # Panics
    ///
    /// When debug assertions are enabled, this function will panic if the requested translation
    /// overlaps with another translation which is not at the same level with the same permissions.
    pub fn map_pages(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        size: usize,
        page_size: PageSize,
        prot: Prot,
    ) -> Result<()> {
        let map_level = if page_size == PageSize::Size1GiB && !MMU_INFO.gigapages {
            PageSize::Size2MiB as usize
        } else {
            page_size as usize
        };
        let page_size = MMU_INFO.page_size[map_level];

        debug_assert!(virt.is_canonical());
        debug_assert!(virt.is_aligned(page_size));
        debug_assert!(phys.is_aligned(page_size));
        debug_assert!(pow2::is_aligned!(size, page_size));

        let mut map_virt = virt;
        let mut map_phys = phys;
        let mut num_pages = size / page_size;

        while num_pages > 0 {
            let mut table = self.top_level.addr.to_virt().as_mut_ptr::<Pte>();
            let mut level = MMU_INFO.max_level;
            loop {
                let table_index = map_virt.index_for(level);
                let entry_ptr = unsafe { table.add(table_index) };
                let mut entry = unsafe { entry_ptr.read_volatile() };

                // Handle parent entries.
                if level != map_level as u32 {
                    if !entry.present() {
                        let page = alloc_page_table();
                        entry = Pte::new(page.addr, PteFlags::PRESENT);
                    }

                    assert!(
                        !entry.huge(),
                        "translation clobbers huge page at {:p}",
                        entry.addr()
                    );

                    entry |= PARENT_FLAGS;
                    unsafe { entry_ptr.write_volatile(entry) };

                    table = entry.addr().to_virt().as_mut_ptr();
                    level -= 1;
                    continue;
                }

                // Set up all the entries in this table.
                let map_pages = cmp::min(num_pages, PTES_PER_TABLE - table_index);
                for i in 0..map_pages {
                    let entry_ptr = unsafe { entry_ptr.add(i) };
                    let entry = unsafe { entry_ptr.read_volatile() };

                    let new_entry = Pte::new(
                        map_phys,
                        MMU_INFO.protmap[prot] | MMU_INFO.pte_flags[map_level] | PteFlags::PRESENT,
                    );

                    // Make sure the entry isn't already present.
                    if entry.present() && entry != new_entry {
                        return Err(Error::AlreadyMapped(entry.addr()));
                    }

                    unsafe { entry_ptr.write_volatile(new_entry) };

                    map_virt += page_size;
                    map_phys += page_size;
                }
                num_pages -= map_pages;
                break;
            }
        }

        self.mapped_pages[map_level] += num_pages;

        Ok(())
    }

    #[inline(always)]
    pub fn switch_to(&self) {
        unsafe {
            asm!(
                "mov cr3, {}",
                in(reg) self.cr3,
                options(nostack, preserves_flags)
            );
        }
    }
}

const PTES_PER_TABLE: usize = PAGE_SIZE / size_of!(Pte);
const MAX_PAGE_LEVEL: usize = 4;

const MAX_LEVELS: usize = 5;
const PAGE_SHIFT_4K: u32 = 12;
const PAGE_SHIFT_2M: u32 = 21;
const PAGE_SHIFT_1G: u32 = 30;
const PAGE_SIZE_4K: usize = 1usize << PAGE_SHIFT_4K;
const PAGE_SIZE_2M: usize = 1usize << PAGE_SHIFT_2M;
const PAGE_SIZE_1G: usize = 1usize << PAGE_SHIFT_1G;
// const PAGE_MASK_4K: usize = PAGE_SIZE_4K - 1;
// const PAGE_MASK_2M: usize = PAGE_SIZE_2M - 1;
// const PAGE_MASK_1G: usize = PAGE_SIZE_1G - 1;

const PARENT_FLAGS: PteFlags = PteFlags::USER.union(PteFlags::WRITE);

struct MmuInfo {
    nx_bit:            PteFlags,
    global_bit:        PteFlags,
    gigapages:         bool,
    bits:              u32,
    levels:            u32,
    max_level:         u32,
    protmap:           ProtMap,
    noncanonical_hole: Range<VirtAddr>,
    pte_flags:         [PteFlags; MAX_LEVELS],
    page_size:         [usize; MAX_LEVELS],
    pcide:             bool,
    max_page_size:     PageSize,
}

static MMU_INFO: BootstrapCell<MmuInfo> = unsafe {
    BootstrapCell::new(MmuInfo {
        nx_bit:            PteFlags::empty(),
        gigapages:         false,
        protmap:           ProtMap::empty(),
        bits:              48,
        levels:            4,
        max_level:         3,
        noncanonical_hole: VirtAddr(1 << 47)..VirtAddr(!0 << 47),
        page_size:         [PAGE_SIZE_4K, PAGE_SIZE_2M, PAGE_SIZE_1G, !0, !0],
        pte_flags:         [
            PteFlags::empty(),
            PteFlags::HUGE,
            PteFlags::HUGE,
            PteFlags::empty(),
            PteFlags::empty(),
        ],
        pcide:             false,
        max_page_size:     PageSize::Size2MiB,
        global_bit:        PteFlags::empty(),
    })
};

impl vm::ArchVm for ThisArch {
    type MdPage = ();

    fn min_user_addr() -> VirtAddr {
        VirtAddr(0x1000)
    }

    fn max_user_addr() -> VirtAddr {
        MMU_INFO.noncanonical_hole.end - 1
    }
}

static PAGING_MODE_REQUEST: limine::PagingModeRequest = limine::PagingModeRequest::new(
    if cfg!(feature = "vm_five-level-paging") {
        limine::PagingMode::FiveLevel
    } else {
        limine::PagingMode::FourLevel
    },
    limine::PagingModeRequestFlags::empty(),
);

static mut INITIAL_PTES: BootstrapCell<[Pte; PTES_PER_TABLE]> =
    unsafe { BootstrapCell::new([Pte::NULL; PTES_PER_TABLE]) };

pub fn init() {
    let mmu_info = unsafe { &mut *BootstrapCell::get_mut_ptr(&MMU_INFO) };

    mmu_info.protmap = ProtMap::new(mmu_info.nx_bit);

    let mut cr4 = Cr4::read();
    let mut efer = Efer::read();

    if CPU_FEATURES[CpuFeat::PAGE_SIZE_1GB] {
        mmu_info.gigapages = true;
        mmu_info.max_page_size = PageSize::Size1GiB;
    }
    if CPU_FEATURES[CpuFeat::EXECUTE_DISABLE] {
        mmu_info.nx_bit |= PteFlags::NO_EXECUTE;
        efer |= Efer::NXE;
    }
    if CPU_FEATURES[CpuFeat::PGE] {
        mmu_info.global_bit = PteFlags::GLOBAL;
        cr4 |= Cr4::PGE;
    }
    if CPU_FEATURES[CpuFeat::PCID] {
        mmu_info.pcide = true;
        cr4 |= Cr4::PCIDE;
    }
    if CPU_FEATURES[CpuFeat::SMAP] {
        cr4 |= Cr4::SMAP;
        log::info!("smap is enabled");
    }
    if CPU_FEATURES[CpuFeat::SMEP] {
        cr4 |= Cr4::SMEP;
        log::info!("smep is enabled");
    }
    if CPU_FEATURES[CpuFeat::UMIP] {
        cr4 |= Cr4::UMIP;
        log::info!("smep is enabled");
    }

    unsafe {
        cr4.write();
        efer.write();
    }

    #[cfg(feature = "vm_five-level-paging")]
    if let Some(resp) = PAGING_MODE_REQUEST.response() {
        if resp.mode() == limine::PagingMode::FiveLevel {
            log::info!("using 5-level paging");
            assert!(cr4.contains(Cr4::LA57));

            mmu_info.levels = 5;
            mmu_info.max_level = 4;
            mmu_info.bits = 57;
            mmu_info.noncanonical_hole = VirtAddr(1 << 56)..VirtAddr(!0 << 56);
        } else {
            assert!(!cr4.contains(Cr4::LA57));
        }
    }

    // Allocate the kernel's top-level PTEs.
    for i in PTES_PER_TABLE / 2..PTES_PER_TABLE {
        let page = alloc_page_table();
        let pte = Pte::new(page.addr, PARENT_FLAGS | PteFlags::PRESENT);
        unsafe {
            BootstrapCell::get_mut_ptr(&INITIAL_PTES)
                .cast::<Pte>()
                .add(i)
                .write(pte);
        }
    }

    KERNEL_HAT.initialize_with(Hat::new(1));
}

struct ProtMap([PteFlags; 16]);

impl Index<Prot> for ProtMap {
    type Output = PteFlags;

    fn index(&self, index: Prot) -> &Self::Output {
        &self.0[index.bits() as usize]
    }
}

impl IndexMut<Prot> for ProtMap {
    fn index_mut(&mut self, index: Prot) -> &mut Self::Output {
        &mut self.0[index.bits() as usize]
    }
}

impl ProtMap {
    const fn empty() -> Self {
        Self([PteFlags::empty(); 16])
    }

    const fn new(nx_bit: PteFlags) -> Self {
        Self([
            /* --- */ nx_bit,
            /* --x */ PteFlags::empty(),
            /* -w- */ PteFlags::WRITE.union(nx_bit),
            /* -wx */ PteFlags::WRITE,
            /* r-- */ nx_bit,
            /* r-x */ PteFlags::empty(),
            /* rw- */ PteFlags::WRITE.union(nx_bit),
            /* rwx */ PteFlags::WRITE,
            /* --- */ nx_bit.union(PteFlags::USER),
            /* --x */ PteFlags::empty().union(PteFlags::USER),
            /* -w- */ PteFlags::WRITE.union(nx_bit).union(PteFlags::USER),
            /* -wx */ PteFlags::WRITE.union(PteFlags::USER),
            /* r-- */ nx_bit.union(PteFlags::USER),
            /* r-x */ PteFlags::empty().union(PteFlags::USER),
            /* rw- */ PteFlags::WRITE.union(nx_bit).union(PteFlags::USER),
            /* rwx */ PteFlags::WRITE.union(PteFlags::USER),
        ])
    }
}

pub static KERNEL_HAT: Lazy<Mutex<Hat>> = Lazy::new(|| unimplemented!());

/// Allocate a new, empty [`PageTable`]
fn alloc_page_table() -> &'static vm::Page {
    let page = vm::Page::alloc(&mut PMAP_QUEUE.lock()).unwrap();
    unsafe { page.addr.to_virt().write_bytes(0, PAGE_SIZE) };
    page
}

static PROT_MAP: Lazy<ProtMap> = Lazy::new(|| {
    let nx_bit = if cpu::CPU_FEATURES[CpuFeat::EXECUTE_DISABLE] {
        PteFlags::NO_EXECUTE
    } else {
        PteFlags::empty()
    };
    ProtMap::new(nx_bit)
});

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PteFlags : u64 {
        const PRESENT             = 1 << 0;
        const WRITE               = 1 << 1;
        const USER                = 1 << 2;
        const CACHE_WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE       = 1 << 4;
        const ACCESSED            = 1 << 5;
        const DIRTY               = 1 << 6;
        const PAT_4K              = 1 << 7;
        const HUGE                = 1 << 7;
        const GLOBAL              = 1 << 8;
        const PAT_HUGE            = 1 << 12;
        const NO_EXECUTE          = 1 << 63;
    }
}

impl From<Prot> for PteFlags {
    fn from(prot: Prot) -> Self {
        PROT_MAP[prot]
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
struct Pte(u64);

impl Pte {
    const NULL: Self = Self(0);

    const ADDR_MASK: u64 = 0x000ffffffffff000;

    const fn new(addr: PhysAddr, flags: PteFlags) -> Pte {
        Self(addr.0 as u64 | flags.bits())
    }

    const fn addr(self) -> PhysAddr {
        PhysAddr::new((self.0 & Self::ADDR_MASK) as usize)
    }

    const fn flags(self) -> PteFlags {
        PteFlags::from_bits_truncate(self.0)
    }

    const fn present(self) -> bool {
        self.flags().contains(PteFlags::PRESENT)
    }

    const fn huge(self) -> bool {
        self.flags().contains(PteFlags::HUGE)
    }
}

impl fmt::Debug for Pte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pte({:p}, {:?})", self.addr(), self.flags())
    }
}

impl ops::BitOrAssign<PteFlags> for Pte {
    fn bitor_assign(&mut self, rhs: PteFlags) {
        self.0 |= rhs.bits();
    }
}
