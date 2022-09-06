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

use core::{alloc::GlobalAlloc, cmp::Ordering, ptr};

use crate::{
    pages_for, pmm,
    vm::{PhysAddr, VirtAddr, PAGE_SIZE},
};

struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        assert!(layout.align() <= PAGE_SIZE);
        let num_frames = pages_for!(layout.size());

        pmm::alloc_frames(num_frames)
            .map_or_else(ptr::null_mut, |phys| phys.to_virtual().as_mut_ptr())
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let old_frames = pages_for!(layout.size());
        let new_frames = pages_for!(new_size);

        match new_frames.cmp(&old_frames) {
            Ordering::Equal => ptr,
            Ordering::Less => {
                let freed_frames = old_frames - new_frames;
                let freed_base = PhysAddr::new(ptr as usize + PAGE_SIZE * new_frames);

                pmm::free_frames(freed_base, freed_frames);

                ptr
            }
            Ordering::Greater => pmm::alloc_frames(new_frames).map_or_else(ptr::null_mut, |phys| {
                let addr = phys.to_virtual().as_mut_ptr::<u8>();
                addr.copy_from(ptr, layout.size());
                pmm::free_frames(VirtAddr::from_ptr(ptr).to_physical(), old_frames);
                addr
            }),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let num_frames = pages_for!(layout.size());

        pmm::free_frames(VirtAddr::from_ptr(ptr).to_physical(), num_frames);
    }
}

#[global_allocator]
static KALLOC: DummyAllocator = DummyAllocator;
