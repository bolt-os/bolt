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

use core::{ptr::NonNull, sync::atomic::compiler_fence};

pub use bolt::arch::vm::with_userspace_access;

pub const PAGE_SIZE: usize = 0x1000;

#[macro_export]
macro_rules! pages_for {
    ($size:expr) => {
        ($size as usize + $crate::vm::PAGE_SIZE - 1) / $crate::vm::PAGE_SIZE
    };
    (type $t:ty) => {
        pages_for!(::core::mem::size_of::<$t>())
    };
}

static mut HHDM_BASE: VirtAddr = VirtAddr(0);

pub unsafe fn set_hhdm(base: VirtAddr) {
    HHDM_BASE = base;
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub const fn new(addr: usize) -> PhysAddr {
        Self(addr)
    }

    pub fn to_virtual(self) -> VirtAddr {
        VirtAddr(unsafe { HHDM_BASE.0 } + self.0)
    }

    pub const fn to_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for PhysAddr {
    fn from(addr: usize) -> PhysAddr {
        Self::new(addr)
    }
}
impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> usize {
        addr.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub const fn new(addr: usize) -> VirtAddr {
        Self(addr)
    }

    pub const fn null() -> VirtAddr {
        Self::new(0)
    }

    pub fn from_ptr<T>(ptr: *const T) -> VirtAddr {
        Self::new(ptr as _)
    }

    pub const fn to_usize(self) -> usize {
        self.0
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub unsafe fn to_physical(self) -> PhysAddr {
        PhysAddr(self.0 - HHDM_BASE.0)
    }

    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    pub unsafe fn as_ref<'a, T: 'a>(self) -> &'a T {
        &*self.as_ptr()
    }

    pub unsafe fn as_mut_ref<'a, T: 'a>(self) -> &'a mut T {
        &mut *self.as_mut_ptr()
    }
}

impl ::core::ops::Add<usize> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl<T> From<*const T> for VirtAddr {
    fn from(ptr: *const T) -> Self {
        Self::new(ptr as _)
    }
}

impl<T> From<*mut T> for VirtAddr {
    fn from(ptr: *mut T) -> Self {
        Self::new(ptr as _)
    }
}

impl From<VirtAddr> for usize {
    fn from(virt: VirtAddr) -> usize {
        virt.to_usize()
    }
}

impl From<VirtAddr> for u64 {
    fn from(virt: VirtAddr) -> u64 {
        virt.to_usize() as _
    }
}

impl<T> From<NonNull<T>> for VirtAddr {
    fn from(ptr: NonNull<T>) -> VirtAddr {
        VirtAddr::new(ptr.addr().get())
    }
}

// bitflags::bitflags! {
//     pub struct MapFlags : u32 {
//         const READ  = 1 << 0;
//         const WRITE = 1 << 1;
//         const EXEC  = 1 << 2;
//         const USER  = 1 << 3;
//     }
// }
//
// #[derive(Clone, Debug, Eq, Hash, PartialEq)]
// pub struct Mapping {
//     virt: VirtAddr,
//     phys: PhysAddr,
//     size: usize,
// }
//
// impl Mapping {
//     pub const fn virt_address(&self) -> VirtAddr {
//         self.virt
//     }
// }
//
// impl PartialOrd for Mapping {
//     fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
//         Some(Self::cmp(self, other))
//     }
// }
//
// impl Ord for Mapping {
//     fn cmp(&self, other: &Self) -> core::cmp::Ordering {
//         self.virt.cmp(&other.virt)
//     }
// }

// pub struct Vmspace {
//     mappings: LinkedList<Mapping>,
//     arch: ArchVmspace,
// }
//
// pub enum MapError {
//
// }
//
// pub type MapResult<T> = Result<T, MapError>;
//
// #[must_use]
// pub struct UnflushedMapping(Mapping);
//
// impl UnflushedMapping {
//     pub fn flush(self) -> Mapping {
//         unsafe { bolt::arch::vm::flush_mapping(&self.0) };
//         self.0
//     }
//
//     pub fn skip_flush(self) -> Mapping {
//         self.0
//     }
// }
//
// bitflags::bitflags! {
//     pub struct Prot : u32 {
//         const READ  = 1 << 0;
//         const WRITE = 1 << 1;
//         const EXEC  = 1 << 2;
//         const USER  = 1 << 3;
//     }
// }
//
// bitflags::bitflags! {
//     pub struct Map : u32 {
//         const ANON  = 1 << 0;
//         const HUGE  = 1 << 1;
//     }
// }
//
// impl Vmspace {
//     pub fn create_mapping(
//         &mut self,
//         virt: VirtAddr,
//         phys: PhysAddr,
//         prot: Prot,
//         flags: Map,
//     ) -> MapResult<UnflushedMapping> {
//         todo!()
//     }
// }

//
//
// struct PhysPage {
//
// }
//
// struct Vnode;
//
// pub enum Object {
//     Anonymous {
//         phys_addr: Option<PhysAddr>,
//     },
//     Vnode {
//         vnode:  Arc<Vnode>,
//         offset: usize,
//     },
//
// }

fn handle_page_fault() {}

pub fn init() {
    use bolt::trap;

    trap::register_handler();
}
