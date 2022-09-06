use core::{alloc::Allocator, ptr::NonNull, sync::atomic::Ordering};

use crate::{
    pmm,
    vm::{PhysAddr, VirtAddr, PAGE_SIZE},
};

use super::{
    asm,
    cpu::{Cr0, Rflags, SMAP_ENABLED},
};

pub fn with_userspace_access<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    // Set Rflags::AC, and get the previous value back.
    // We can only execute `stac` when SMAP is enabled, so gate it behind a check.
    // Default to `true` if it's disabled so the terminating `clac` will not be executed either.
    let prev_ac = SMAP_ENABLED
        .load(Ordering::Relaxed)
        // SAFETY: I mean, it's not *our* memory.
        .then(|| unsafe { asm::push_stac() })
        .unwrap_or(true);

    let result = f();

    if !prev_ac {
        unsafe { asm::clac() };
    }

    result
}

bitflags::bitflags! {
    #[repr(transparent)]
    struct PteFlags : usize {
        const PRESENT       = 1 << 0;

    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
struct PageTableEntry(usize);

impl PageTableEntry {
    const fn addr(self) -> PhysAddr {
        PhysAddr::new(self.0 & (!0 << 12))
    }

    const fn is_present(self) -> bool {
        self.0 & 0x1 != 0
    }
}

pub fn init() {
    let mut cr0 = Cr0::read();

    cr0 |= Cr0::WP;
}

struct PageTableAllocator;

unsafe impl Allocator for PageTableAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        self.allocate_zeroed(layout)
    }

    fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let addr = pmm::alloc_frames(1)
            .ok_or(core::alloc::AllocError)?
            .to_virtual();
        unsafe {
            addr.as_mut_ptr::<u8>().write_bytes(0, PAGE_SIZE);
            Ok(NonNull::slice_from_raw_parts(
                addr.as_mut_ref::<u8>().into(),
                PAGE_SIZE,
            ))
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        debug_assert_eq!(layout.size(), PAGE_SIZE);
        pmm::free_frames(VirtAddr::from(ptr).to_physical(), 1);
    }
}

#[repr(transparent)]
struct PageTable {
    entries: [PageTableEntry; 512],
}

pub struct ArchVmspace {
    pml4: Box<PageTable, PageTableAllocator>,
}
