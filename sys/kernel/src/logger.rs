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

use core::fmt;

use log::{Level, Log};

use crate::{
    arch,
    cpu::this_cpu,
    sync::{mutex::MutexKind, Mutex, MutexGuard},
};

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        arch::print_str(s);
        Ok(())
    }
}

static WRITER: Mutex<Writer> = Mutex::new(MutexKind::Spin, Writer);

pub fn lock<'a>() -> MutexGuard<'a, Writer> {
    WRITER.lock()
}

pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    <Writer as fmt::Write>::write_fmt(&mut writer, args).ok();
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::logger::_print(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($t:tt)*) => { $crate::print!("{}\n", format_args!($($t)*)) };
}

#[macro_export]
macro_rules! dbg {
    ($e:expr) => {{
        let e = $e;
        println!(
            concat!("[", file!(), ":", line!(), "] ", stringify!($e), " = {:#?}"),
            e
        );
        e
    }};
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {}

    fn log(&self, record: &log::Record) {
        let (color, name) = match record.level() {
            Level::Error => ("\x1b[38;2;204;036;029m", "error"),
            Level::Warn => ("\x1b[38;2;215;153;033m", "warn "),
            Level::Info => ("\x1b[38;2;249;245;215m", "info "),
            Level::Debug => ("\x1b[38;2;152;151;026m", "debug"),
            Level::Trace => ("\x1b[38;5;46m", "trace"),
        };
        let cpuid = unsafe { (*this_cpu()).cpu_id };
        let target = record.target().trim_start_matches("bolt_kernel::");

        println!(
            "{color}[{name}] cpu{cpuid}: {target}: {}\x1b[m",
            record.args()
        );
    }
}

pub fn init() {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}
