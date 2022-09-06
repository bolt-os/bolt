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

use bolt::vm::{PhysAddr, PAGE_SIZE};
use core::ptr::NonNull;

use spin::mutex::SpinMutex;

use crate::{pages_for, vm::VirtAddr};

struct PhysRegion {
    base: usize,
    len: usize,
    next: Option<NonNull<PhysRegion>>,
    prev: Option<NonNull<PhysRegion>>,
}

struct LinkedListPmm {
    head: Option<NonNull<PhysRegion>>,
    tail: Option<NonNull<PhysRegion>>,
}

unsafe impl Send for LinkedListPmm {}

impl LinkedListPmm {
    const fn new() -> LinkedListPmm {
        Self {
            head: None,
            tail: None,
        }
    }

    fn remove_region(&mut self, region: &PhysRegion) {
        if let Some(mut prev) = region.prev {
            unsafe { prev.as_mut().next = region.next };
        } else {
            self.head = region.next;
        }

        if let Some(mut next) = region.next {
            unsafe { next.as_mut().prev = region.prev };
        } else {
            self.tail = region.prev;
        }
    }

    fn insert_region(&mut self, mut base: usize, mut len: usize) {
        let mut prev = self.tail;
        let mut next = None;

        while let Some(region) = prev {
            let region = unsafe { region.as_ref() };

            if region.base < base {
                break;
            }

            next = prev;
            prev = region.prev;
        }

        if let Some(prevp) = prev {
            let prevp = unsafe { prevp.as_ref() };
            let prev_end = prevp.base + PAGE_SIZE * prevp.len;
            assert!(prev_end <= base, "overlapping regions");

            if prev_end == base {
                base = prevp.base;
                len += prevp.len;
                prev = prevp.prev;
            }
        }
        if let Some(nextp) = next {
            let nextp = unsafe { nextp.as_ref() };
            let new_end = base + PAGE_SIZE * len;
            assert!(new_end <= nextp.base, "overlapping regions");

            if new_end == nextp.base {
                len += nextp.len;
                next = nextp.next;
            }
        }

        let new_node = unsafe {
            let ptr = PhysAddr::new(base).to_virtual().as_mut_ptr();

            core::ptr::write(
                ptr,
                PhysRegion {
                    base,
                    len,
                    next,
                    prev,
                },
            );

            Some(NonNull::new_unchecked(ptr))
        };

        if let Some(mut prev) = prev {
            unsafe { prev.as_mut().next = new_node };
        } else {
            self.head = new_node;
        }
        if let Some(mut next) = next {
            unsafe { next.as_mut().prev = new_node };
        } else {
            self.tail = new_node;
        }
    }
}

static PHYSMAP: SpinMutex<LinkedListPmm> = SpinMutex::new(LinkedListPmm::new());

pub fn alloc_frames(num_frames: usize) -> Option<PhysAddr> {
    let mut physmap = PHYSMAP.lock();
    let mut nodep = physmap.tail;

    while let Some(mut node) = nodep {
        let node = unsafe { node.as_mut() };

        if node.len >= num_frames {
            node.len -= num_frames;
            if node.len == 0 {
                physmap.remove_region(node);
            }

            return Some(PhysAddr::new(node.base + PAGE_SIZE * node.len));
        }

        nodep = node.prev;
    }

    None
}

pub fn free_frames(base: PhysAddr, num_frames: usize) {
    PHYSMAP.lock().insert_region(base.to_usize(), num_frames);
}

pub fn print_mmap() {
    let physmap = PHYSMAP.lock();
    let mut nodep = physmap.head;

    while let Some(node) = nodep {
        let node = unsafe { node.as_ref() };

        println!(
            "  {:#018x} -> {:#018x}, {} pages",
            node.base,
            node.base + PAGE_SIZE * node.len,
            node.len
        );

        nodep = node.next;
    }
}

/// Physical Frame Allocator
pub struct FrameAllocator;

unsafe impl core::alloc::Allocator for FrameAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        debug_assert!(layout.align() == PAGE_SIZE);
        debug_assert!(layout.size() % PAGE_SIZE == 0);

        alloc_frames(pages_for!(layout.size()))
            .map(|phys| unsafe {
                let ptr = phys.to_virtual().as_mut_ptr::<u8>();
                ptr.write_bytes(0, layout.size());
                NonNull::slice_from_raw_parts(NonNull::new_unchecked(ptr), layout.size())
            })
            .ok_or(core::alloc::AllocError)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let base = VirtAddr::from(ptr.as_ptr()).to_physical();
        let num_frames = pages_for!(layout.size());

        free_frames(base, num_frames);
    }
}
