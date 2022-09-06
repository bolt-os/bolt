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

use std::{path::{PathBuf, Path}, ffi::OsStr, process::Command};

pub mod nasm;

#[macro_export]
macro_rules! getenv {
    ($var:literal) => {
        match ::std::env::var($var) {
            Ok(var) => var,
            Err(err) => ::std::panic!("environment variable `{}` not defined: {}", $var, err),
        }
    };
}

#[macro_export]
macro_rules! pathcat {
    ($($component:expr),+ $(,)?) => {{
        let mut path = ::std::path::PathBuf::new();
        $(path.push(&$component);)+
        path
    }};
}

pub type Result<T> = std::result::Result<T, crate::Error>;

#[derive(Debug)]
pub enum Error {
    CompilerError,
    IoError(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerError => write!(f, "compilation failure"),
            Self::IoError(err) => <std::io::Error as std::fmt::Display>::fmt(err, f),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}


pub fn find_files<P, I, S>(src: P, ext: S) -> Result<Vec<PathBuf>>
where
    P: IntoIterator<Item = I>,
    I: AsRef<Path>,
    S: AsRef<OsStr>,
{
    fn find_files_inner<P, S>(src: P, ext: S, buf: &mut Vec<PathBuf>) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        for dirent in std::fs::read_dir(&src)? {
            let entry = dirent?.path();

            if entry.is_file() && entry.extension() == Some(ext.as_ref()) {
                buf.push(entry);
            } else if entry.is_dir() {
                find_files_inner(entry, ext.as_ref(), buf)?;
            }
        }

        Ok(())
    }

    let mut buf = vec![];

    for path in src {
        find_files_inner(path, &ext, &mut buf)?;
    }

    Ok(buf)
}

pub fn create_archive<N, I, F>(name: N, input: I) -> Result<()>
where
    N: AsRef<Path>,
    I: IntoIterator<Item = F>,
    F: AsRef<OsStr>,
{
    let output = pathcat!(
        getenv!("OUT_DIR"),
        format!("lib{}.a", name.as_ref().display())
    );
    let mut cmd = Command::new("llvm-ar");

    cmd.arg("crus");
    cmd.arg(output);
    cmd.args(input);

    if cmd.spawn()?.wait()?.success() {
        println!("cargo:rustc-link-lib=static={}", name.as_ref().display());
        Ok(())
    } else {
        Err(Error::CompilerError)
    }
}
