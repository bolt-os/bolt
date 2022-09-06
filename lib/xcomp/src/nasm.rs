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

use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{create_archive, find_files, getenv, Error, Result};

macro_rules! str_enum {
    (
        $(#[$enum_meta:meta])*
        $enum_vis:vis enum $enum_name:ident {
            $(
                $(#[$var_meta:meta])*
                $var_name:ident = $var_str:literal
            ),*$(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        $enum_vis enum $enum_name {
            $(
                $(#[$var_meta])*
                $var_name
            ),*
        }

        impl $enum_name {
            pub const fn as_str(&self) -> &str {
                match self {
                    $(Self::$var_name => $var_str),*
                }
            }
        }
    };
}

str_enum! {
    pub enum OutputFormat {
        Bin = "bin",
        IntelHex = "ith",
        SRecords = "srec",
        Aout = "aout",
        BsdAout = "aoutb",
        Coff = "coff",
        Elf32 = "elf32",
        Elf64 = "elf64",
        Elfx32 = "elfx32",
        As86 = "as86",
        Obj = "obj",
        Win32 = "win32",
        Win64 = "win64",
        Ieee = "ieee",
        Macho32 = "macho32",
        Macho64 = "macho64",
        Dbg = "dbg",
        Elf = "elf",
        Macho = "macho",
        Win = "win",
    }
}

pub struct NasmConfig {
    nasm_path: PathBuf,
    nasm_ext: String,
    arguments: Vec<OsString>,
    archive_name: String,
}

impl NasmConfig {
    pub fn new(ofmt: OutputFormat) -> NasmConfig {
        Self {
            nasm_path: "nasm".into(),
            nasm_ext: "asm".into(),
            archive_name: "nasm".into(),
            arguments: vec![format!("-f{}", ofmt.as_str()).into()],
        }
    }

    pub fn arg<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Self {
        self.arguments.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, A>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        self.arguments
            .extend(args.into_iter().map(|arg| arg.as_ref().to_os_string()));
        self
    }
}

pub fn assemble_file<P: AsRef<Path>>(input: P, conf: &NasmConfig) -> Result<PathBuf> {
    let input_file = input.as_ref();
    let output_file = {
        let out_dir = PathBuf::from(getenv!("OUT_DIR"));
        let mut path = PathBuf::new();

        path.push(out_dir);
        path.push(
            input_file
                .with_extension("o")
                .strip_prefix(getenv!("CARGO_MANIFEST_DIR"))
                .unwrap(),
        );

        path
    };
    let mkdep_file = output_file.with_extension("d");

    if !output_file.parent().unwrap().exists() {
        std::fs::create_dir_all(output_file.parent().unwrap())?;
    }

    println!("cargo:rerun-if-changed={}", input_file.display());

    let mut cmd = Command::new(&conf.nasm_path);
    cmd.args(&conf.arguments);
    cmd.args(["-MP", "-MD"]);
    cmd.arg(&mkdep_file);
    cmd.arg("-o");
    cmd.arg(&output_file);
    cmd.arg(&input_file);

    let args = cmd
        .get_args()
        .map(|a| a.to_string_lossy())
        .collect::<Vec<Cow<str>>>()
        .join(" ");
    println!("nasm {args}");

    if cmd.spawn()?.wait()?.success() {
        Ok(output_file)
    } else {
        Err(Error::CompilerError)
    }
}

pub fn run<P, I>(dir: P, conf: &NasmConfig) -> Result<()>
where
    P: IntoIterator<Item = I>,
    I: AsRef<Path>,
{
    let sources = find_files(dir, &conf.nasm_ext)?;
    let mut objects = vec![];

    for src in sources {
        if !src.exists() {
            continue;
        }

        objects.push(assemble_file(src, conf)?);
    }

    if !objects.is_empty() {
        create_archive(&conf.archive_name, &objects)?;
        println!("cargo:rustc-link-search=native={}", getenv!("OUT_DIR"));
    }

    Ok(())
}
