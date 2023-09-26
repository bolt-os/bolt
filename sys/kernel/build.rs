#![feature(exit_status_error)]

use std::{env, error::Error, fs, path::PathBuf, process::Command};

const LAI_SOURCES: &[&str] = &[
    "core/error.c",
    "core/eval.c",
    "core/exec.c",
    "core/exec-operand.c",
    "core/libc.c",
    "core/ns.c",
    "core/object.c",
    "core/opregion.c",
    "core/os_methods.c",
    "core/variable.c",
    "core/vsnprintf.c",
    "helpers/pc-bios.c",
    "helpers/pci.c",
    "helpers/resource.c",
    "helpers/sci.c",
    "helpers/pm.c",
    "drivers/ec.c",
    "drivers/timer.c",
];

fn get_env_path(key: &str) -> Option<PathBuf> {
    match env::var(key) {
        Ok(path) => Some(path.into()),
        Err(env::VarError::NotUnicode(path)) => Some(path.into()),
        Err(env::VarError::NotPresent) => None,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let arch = env::var("CARGO_CFG_TARGET_ARCH")?;

    println!("cargo:rustc-env=BOLT_KERNEL_TARGET_ARCH={arch}");

    let cc = get_env_path("CC").unwrap_or_else(|| "clang".into());
    println!("cargo:rerun-if-env-changed=CC");

    let cc_is_clang = Command::new(&cc)
        .arg("--version")
        .output()?
        .stdout
        .windows(5)
        .any(|s| s == b"clang");
    println!("CC_IS_CLANG={cc_is_clang}");

    let mut target_cflags = vec![];

    if cc_is_clang {
        target_cflags.extend_from_slice(&["-target", format!("{arch}-elf").leak()]);
    }

    #[rustfmt::skip]
    target_cflags.extend([
        "-ffreestanding",
        "-nostdinc",

        "-ffunction-sections",
        "-fdata-sections",
    ]);

    match arch.as_str() {
        "x86_64" => {
            target_cflags.extend_from_slice(&["-mno-mmx", "-mno-sse", "-mno-red-zone"]);
        }
        "riscv64" => {
            // Clang does not recognize Zicsr or Zifencei in `-march` until Clang 17.
            // GCC requires them to be specified in order to use the instructions.
            if cc_is_clang {
                target_cflags.push("-march=rv64imac");
            } else {
                target_cflags.push("-march=rv64imac_zicsr_zifencei");
            }
            target_cflags.push("-mabi=lp64");
        }
        _ => panic!("unknown architecture: `{arch}`"),
    };

    {
        // Run the C preprocessor on all assembly files (.s or .S) in the
        // relevant architecture directory.

        let _include_dir = format!("arch/{arch}/include");

        for entry in walkdir::WalkDir::new(format!("arch/{arch}/src")).follow_links(true) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            if entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("s"))
            {
                let output_path = out_dir.clone().join(entry.path());
                fs::create_dir_all(output_path.parent().unwrap())?;

                #[rustfmt::skip]
                Command::new(&cc)
                    .args(&target_cflags)
                    .args([
                        "-x", "assembler-with-cpp",
                        "-E",
                        "-o"
                    ])
                    .arg(output_path)
                    .arg(entry.path())
                    .spawn()?
                    .wait()?
                    .exit_ok()?;

                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }

    println!("cargo:rustc-cfg=kernel");

    let lai_dir = env::var("BOLT_THIRD_PARTY_lai").unwrap();
    let lai_include_dir = PathBuf::from(&lai_dir).join("include");

    let mut lai_build = cc::Build::new();
    let mut lai_bindgen = bindgen::builder();

    lai_build
        .compiler(&cc)
        .archiver("llvm-ar")
        .include(&lai_include_dir)
        .flag("-ffunction-sections")
        .flag("-ffreestanding")
        .pic(true)
        .files(LAI_SOURCES.iter().map(|p| format!("{lai_dir}/{p}")));

    match arch.as_str() {
        "x86_64" => {
            lai_build.target("x86_64-unknown-none");
            lai_build.flag("-mno-mmx");
            lai_build.flag("-mno-sse");
            lai_build.flag("-mno-red-zone");
            lai_bindgen = lai_bindgen.clang_arg("--target=x86_64-unknown-none");
        }
        "riscv64" => {
            lai_build.target("riscv64");
            lai_build.flag("-march=rv64imac");
            lai_build.flag("-mabi=lp64");
            lai_bindgen =
                lai_bindgen.clang_args(["--target=riscv64", "-march=rv64imac", "-mabi=lp64"]);
        }
        _ => panic!("unknown architecture."),
    }

    lai_build.compile("lai");

    // Run bindgen.

    println!("cargo:rerun-if-changed=lai-bindgen.h");
    let bindings = lai_bindgen
        .header("lai-bindgen.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg(format!("-I{}", lai_include_dir.display()))
        .generate_inline_functions(true)
        .use_core()
        .generate()?;

    bindings.write_to_file(PathBuf::from(env::var("OUT_DIR")?).join("lai.rs"))?;

    println!("cargo:rustc-link-arg=--no-dynamic-linker");

    Ok(())
}
