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

use super::cpu::{self, CPU_FEATURES};
use crate::{
    alloc::sync::Arc,
    arch::ThisArch,
    spl::Ipl,
    sync::{lazy::Lazy, mutex::MutexKind, Mutex},
    util::size_of,
    vm::{self, page::PMAP_QUEUE, PhysAddr, Prot, VirtAddr, PAGE_SIZE},
};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    AlreadyMapped(PhysAddr),
}

impl VirtAddr {
    const fn index_for(self, level: u32) -> usize {
        self.0 >> (12 + 9 * level) & 0x1ff
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSize {
    Normal = 0,
    Mega   = 1,
    Giga   = 2,
}

/// Hardware Address Translation Context
///
/// # Kernel HAT
pub struct Hat {
    /// Top-level page table
    top_level:    *mut Pte,
    /// Tracks the number of pages of each size mapped into the address space
    mapped_pages: [usize; MAX_PAGE_LEVEL + 1],
}

unsafe impl Send for Hat {}

impl Hat {
    pub fn new(vmspace: Option<&Arc<vm::Vmspace>>) -> Mutex<Hat> {
        let page = alloc_page_table();
        Self::with_top_level(page.addr, vmspace)
    }

    pub fn with_top_level(top_level: PhysAddr, _vmspace: Option<&Arc<vm::Vmspace>>) -> Mutex<Hat> {
        Mutex::new(MutexKind::Default, Ipl::VM, Hat {
            top_level:    top_level.to_virt().as_mut_ptr(),
            mapped_pages: [0; MAX_PAGE_LEVEL + 1],
        })
    }

    pub(super) fn cr3(&self) -> usize {
        unsafe { VirtAddr::new(self.top_level.addr()).to_phys().addr() }
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
        let mmu_info = unsafe { &MMU_INFO };
        let map_level = if page_size == PageSize::Giga && !mmu_info.gigapages {
            PageSize::Mega as usize
        } else {
            page_size as usize
        };
        let page_size = mmu_info.page_size[map_level];

        debug_assert!(!mmu_info.noncanonical_hole.contains(&virt));
        debug_assert!(virt.is_aligned(page_size));
        debug_assert!(phys.is_aligned(page_size));
        debug_assert!(size & (page_size - 1) == 0);

        let mut map_virt = virt;
        let mut map_phys = phys;
        let mut num_pages = size / page_size;

        while num_pages > 0 {
            let mut table = self.top_level;
            let mut level = mmu_info.max_level;
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
                let map_pages = cmp::min(num_pages, 512 - table_index);
                for i in 0..map_pages {
                    let entry_ptr = unsafe { entry_ptr.add(i) };
                    let entry = unsafe { entry_ptr.read_volatile() };

                    let new_entry = Pte::new(
                        map_phys,
                        mmu_info.protmap[prot] | mmu_info.pte_flags[map_level] | PteFlags::PRESENT,
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

    pub fn switch_to(&self) {
        unsafe {
            let cr3 = VirtAddr::new(self.top_level.addr()).to_phys().0;
            asm!(
                "
                    mov     cr3, {}
                ",
                in(reg) cr3,
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
    gigapages:         bool,
    bits:              u32,
    levels:            u32,
    max_level:         u32,
    protmap:           ProtMap,
    noncanonical_hole: Range<VirtAddr>,
    pte_flags:         [PteFlags; MAX_LEVELS],
    page_size:         [usize; MAX_LEVELS],
}

static mut MMU_INFO: MmuInfo = MmuInfo {
    nx_bit:            PteFlags::empty(),
    gigapages:         false,
    protmap:           ProtMap::empty(),
    bits:              48,
    levels:            4,
    max_level:         3,
    noncanonical_hole: VirtAddr(0x0000800000000000)..VirtAddr(0xffff800000000000),
    page_size:         [PAGE_SIZE_4K, PAGE_SIZE_2M, PAGE_SIZE_1G, !0, !0],
    pte_flags:         [
        PteFlags::empty(),
        PteFlags::HUGE,
        PteFlags::HUGE,
        PteFlags::empty(),
        PteFlags::empty(),
    ],
};

impl vm::ArchVm for ThisArch {
    type MdPage = ();
}

#[cfg(feature = "vm_five-level-paging")]
static FIVE_LEVEL_PAGING_REQUEST: limine::FiveLevelPagingRequest =
    limine::FiveLevelPagingRequest::new();

pub fn init() {
    let mut mmu_info = unsafe { &mut MMU_INFO };

    mmu_info
        .nx_bit
        .set(PteFlags::NO_EXECUTE, CPU_FEATURES[CpuFeat::EXECUTE_DISABLE]);
    mmu_info.gigapages = CPU_FEATURES[CpuFeat::EXECUTE_DISABLE];
    mmu_info.protmap = ProtMap::new(mmu_info.nx_bit);

    #[cfg(feature = "vm_five-level-paging")]
    if FIVE_LEVEL_PAGING_REQUEST.has_response() {
        mmu_info.levels = 5;
        mmu_info.max_level = 4;
        mmu_info.bits = 57;
        mmu_info.noncanonical_hole = VirtAddr(0x010000000000000)..VirtAddr(0xff00000000000000);

        log::info!("using 5-level paging");
    }

    let root_table = alloc_page_table();

    let table = root_table.addr.to_virt().as_mut_ptr::<Pte>();
    for i in 256..512 {
        let page = alloc_page_table();

        unsafe {
            table
                .add(i)
                .write_volatile(Pte::new(page.addr, PteFlags::PRESENT));
        }
    }

    let kern_hat = Hat::with_top_level(root_table.addr, None);
    KERNEL_HAT.initialize_with(kern_hat);
}

struct ProtMap([PteFlags; 8]);

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
        Self([PteFlags::empty(); 8])
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
        ])
    }
}

pub static KERNEL_HAT: Lazy<Mutex<Hat>> = Lazy::new(|| Hat::new(None));

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
        self.0 |= rhs.bits;
    }
}
