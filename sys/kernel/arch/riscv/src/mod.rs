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

mod cpu;
pub mod trap;
pub mod vm;

use bolt::vm::VirtAddr;

pub fn evil_putstr(s: &str) {
    let uart_out = 0x10000000 as *mut u8;

    for c in s.bytes() {
        unsafe { uart_out.write_volatile(c) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn _start(bootinfo: &'static spark::Bootinfo) -> ! {
    println!("ayyyyy!");
    println!("{:#?}", bootinfo);

    asm!("csrc sstatus, {}", in(reg) !cpu::Sstatus::SPP.bits());

    bolt::logger::init();

    let hhdm_base = HHDM_REQUEST.response().expect("no hhdm :(").base();
    bolt::vm::set_hhdm(VirtAddr::new(hhdm_base));

    for region in bootinfo.free_list.regions() {
        bolt::pmm::free_frames(region.base.into(), region.num_frames);
    }

    let kern_file =
        core::slice::from_raw_parts(bootinfo.kern_file_ptr, bootinfo.kern_file_len).to_vec();
    bolt::panic::register_executable(kern_file).unwrap();

    trap::init();
    bolt::pmm::print_mmap();

    static BOOTLOADER_INFO_REQUEST: limine::BootloaderInfoRequest =
        limine::BootloaderInfoRequest::new();
    let bootloader_info = BOOTLOADER_INFO_REQUEST.response().unwrap();
    log::info!(
        "booted by: {} v{}",
        bootloader_info.brand(),
        bootloader_info.version()
    );

    static HHDM_REQUEST: limine::HhdmRequest = limine::HhdmRequest::new();
    let hhdm = HHDM_REQUEST.response().unwrap();
    log::info!("hhdm base @ {:#018x}", hhdm.base());

    static DTB_REQUEST: limine::DtbRequest = limine::DtbRequest::new();
    println!("dtb @ {:p}", DTB_REQUEST.response().unwrap().dtb_ptr);

    println!("dying on bsp");
    loop {
        core::hint::spin_loop();
    }
}
