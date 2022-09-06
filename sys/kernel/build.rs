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

use std::{env, error::Error, ffi::OsStr, path::Path, str::FromStr};

macro_rules! pathcat {
    ($($component:expr),+$(,)?) => {{
        let mut path = ::std::path::PathBuf::new();
        $(path.push(&$component);)+
        path
    }};
}

macro_rules! watch {
    ($($path:expr),*) => {
        $(println!("cargo:rerun-if-changed={}", ::std::convert::AsRef::<Path>::as_ref(&$path).display());)*
    };
}

macro_rules! link_arg {
    ($($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg={}", $flag);)*
    };
    (bin $bin:expr; $($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg-bin={}={}",
            $bin, ::std::convert::AsRef::<Path>::as_ref(&$flag).display());)*
    };
    (bins $($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg-bins={}", $flag);)*
    };
    (tests $($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg-bins={}", $flag);)*
    };
    (examples $($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg-bins={}", $flag);)*
    };
    (benches $($flag:expr),*$(,)?) => {
        $(println!("cargo:rustc-link-arg-bins={}", $flag);)*
    };
}

struct BuildError {
    message: String,
}

impl From<env::VarError> for BuildError {
    fn from(error: env::VarError) -> Self {
        match error {
            env::VarError::NotPresent => todo!(),
            env::VarError::NotUnicode(_) => todo!(),
        }
    }
}

impl std::fmt::Debug for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "build.rs: {}", self.message)
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "build.rs: {}", self.message)
    }
}

impl std::error::Error for BuildError {}

fn env_var<K: AsRef<OsStr>>(key: K) -> Result<String, BuildError> {
    std::env::var(&key).map_err(|var_err| {
        let message = match var_err {
            env::VarError::NotPresent => "not present",
            env::VarError::NotUnicode(_) => "not valid unicode",
        };

        BuildError {
            message: format!(
                "environment variable `{}` {message}",
                key.as_ref().to_string_lossy()
            ),
        }
    })
}

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let out_dir = env_var("OUT_DIR")?;
    let build_arch = BuildArch::from_str(&env::var("CARGO_CFG_TARGET_ARCH")?)?;

    let kern_root = env::var("CARGO_MANIFEST_DIR")?;
    let arch_root = pathcat!(kern_root, "arch", build_arch);

    let _kern_src = pathcat!(kern_root, "kern");
    let arch_src = pathcat!(arch_root, "src");

    // The build system needs to know which directory cargo emits artifacts into.
    // The path may change between any two invocations of `cargo`, so we create
    // the `.outdir` file, which always contains the most recent $OUT_DIR path.
    std::fs::write(pathcat!(kern_root, ".outdir"), out_dir)?;

    println!("cargo:rustc-env=BUILD_ARCH={build_arch}");

    // Tell the linker which linker script to use.
    let linker_script = pathcat!(arch_root, "conf", "linker.ld");
    link_arg!(bin "boltk"; "-T", linker_script);
    watch!(linker_script);

    link_arg!(
        "--gc-sections",
        "--error-unresolved-symbols",
        "--fatal-warnings"
    );

    if build_arch == BuildArch::X86_64 {
        use xcomp::nasm;

        let mut config = nasm::NasmConfig::new(nasm::OutputFormat::Elf64);

        config.args(["-gdwarf", "-w+all", "-w+error"]);

        nasm::run([&arch_src], &config)?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BuildArch {
    X86_64,
    Riscv,
}

impl FromStr for BuildArch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64" => Ok(Self::X86_64),
            "riscv64" => Ok(Self::Riscv),
            _ => Err(format!("{s} is not a supported architecture")),
        }
    }
}

impl AsRef<Path> for BuildArch {
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

impl BuildArch {
    fn as_str(&self) -> &str {
        match self {
            BuildArch::X86_64 => "x86_64",
            BuildArch::Riscv => "riscv",
        }
    }
}

impl std::fmt::Display for BuildArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
