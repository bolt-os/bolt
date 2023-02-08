use crate::{
    pathcat,
    CommandExt,
    Session,
    XtaskArgs,
};
use anyhow::Context;
use std::{env, fs, process::Command, time::Instant};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum BuildWhat {
    Kernel,
    Userspace,
    All,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct BuildArgs {
    #[arg(default_value = "all")]
    build_what: BuildWhat,

    #[clap(flatten)]
    pub xtask_args: XtaskArgs,

    #[arg(long)]
    pub no_image: bool,

    #[arg(long)]
    pub timing: bool,
}

impl BuildArgs {
    pub fn xtask_args(&self) -> &XtaskArgs {
        &self.xtask_args
    }
}

mod third_party {
    use crate::{pathcat, CommandExt, Session};
    use anyhow::{anyhow, Context};
    use std::{ffi::OsStr, fs, path::PathBuf, process::Command, time::Duration};

    pub fn host_install_git<I>(
        session: &mut Session,
        name: &str,
        url: &str,
        args: I,
    ) -> anyhow::Result<(PathBuf, bool)>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        let ts_file = pathcat!(session.build_dir, "third-party", ".git.updates", name);
        let have_recent = fs::metadata(&ts_file).is_ok_and(|meta| {
            meta.modified().is_ok_and(|modified| {
                modified.elapsed().is_ok_and(|time_since_clone| {
                    time_since_clone < Duration::from_secs(60 * 60 * 24)
                })
            })
        });

        let pkg_dir = pathcat!(session.build_dir, "third-party", name);

        if have_recent {
            return Ok((pkg_dir, true));
        }

        // Delete the old version, if it exists.
        if pkg_dir.exists() {
            fs::remove_dir_all(&pkg_dir)?;
        }

        let mut git = Command::new("git");

        git.args(["clone", url]);
        git.arg(&pkg_dir);
        git.args(args);

        git.log_command();
        git.spawn()?.wait()?.exit_ok()?;

        let error_context =
            || anyhow!("failed to create timestamp file for third-party git package: `{name}`");

        fs::create_dir_all(ts_file.parent().unwrap()).with_context(error_context)?;
        fs::write(ts_file, ".").with_context(error_context)?;

        Ok((pkg_dir, false))
    }
}

#[derive(clap::Args, Clone, Debug)]
pub struct BuildConfig {}

pub fn main(session: &mut Session, args: &mut BuildArgs) -> anyhow::Result<()> {
    let start_time = args.timing.then(Instant::now);

    let (build_kernel, _build_userspace) = match args.build_what {
        BuildWhat::All => (true, true),
        BuildWhat::Kernel => (true, false),
        BuildWhat::Userspace => (false, true),
    };

    if build_kernel {
        let mut cargo = session.cargo();

        cargo.arg("build");

        cargo.args([
            "--manifest-path",
            &format!("{}/sys/kernel/Cargo.toml", env::current_dir()?.display()),
        ]);

        cargo.arg("--target");
        cargo.arg(&*session.kernel_rust_target());

        cargo.arg("--target-dir");
        cargo.arg(&session.cargo_target_dir);

        if args.xtask_args.release {
            cargo.arg("--release");
        }

        if args.timing {
            cargo.arg("--timings");
        }

        if session.target.name == "riscv" {
            cargo.args([
                "-Zbuild-std=alloc,compiler_builtins,core",
                "-Zbuild-std-features=compiler-builtins-mem",
            ]);
        }

        let linker_scipt = pathcat!(session.sys_arch_dir, "conf", "linker.ld");
        cargo.env(
            "RUSTFLAGS",
            format!("-Clink-arg={}", linker_scipt.display()),
        );

        cargo.spawn()?.wait()?.exit_ok()?;
    }

    if let Some(build_time) = start_time.as_ref().map(Instant::elapsed) {
        println!("build time: {build_time:?}");
    }

    let sysroot_boot = pathcat!(session.sysroot_dir, "boot");

    let src = pathcat!(session.kernel_target_dir, "bolt-kernel");
    let dst = pathcat!(sysroot_boot, "boltk");
    fs::create_dir_all(dst.parent().unwrap())?;
    if dst.symlink_metadata().is_err() {
        crate::symlink(src, dst).with_context(|| "failed to symlink")?;
    }

    // Userspace

    // Create a bootable image.
    if !args.no_image {
        // Fetch a bootloader.
        let limine_dir = match session.target.name {
            "x86_64" => {
                let (limine_dir, fresh) = third_party::host_install_git(
                    session,
                    "limine",
                    "https://github.com/limine-bootloader/limine.git",
                    ["--depth=1", "--branch=v3.0-branch-binary"],
                )?;
                if !fresh {
                    let uefi_esp = pathcat!(session.sysroot_dir, "boot", "EFI", "BOOT");
                    fs::create_dir_all(&uefi_esp)?;
                    crate::symlink(pathcat!(limine_dir, "BOOTX64.EFI"), &uefi_esp)?;
                    for path in ["limine-cd.bin", "limine-cd-efi.bin", "limine.sys"] {
                        crate::symlink(pathcat!(limine_dir, path), &sysroot_boot)?;
                    }

                    let mut make = Command::new("make");
                    make.current_dir(&limine_dir);
                    make.arg("limine-deploy");

                    make.log_command();
                    make.spawn()?.wait()?.exit_ok()?;
                }
                crate::symlink(
                    pathcat!(session.sys_arch_dir, "conf", "limine.cfg"),
                    &sysroot_boot,
                )?;

                Some(limine_dir)
            }
            _ => todo!(),
        };

        println!("creating ISO image");
        let image = pathcat!(
            session.output_dir,
            format!("{}.iso", args.xtask_args().build_name())
        );
        let mut xorriso = Command::new("xorriso");

        xorriso.args(["-as", "mkisofs", "-f"]);
        if session.target.name == "x86_64" {
            xorriso.args([
                "-b",
                "boot/limine-cd.bin",
                "-no-emul-boot",
                "-boot-load-size",
                "4",
                "-boot-info-table",
                "--efi-boot",
                "boot/limine-cd-efi.bin",
                "-efi-boot-part",
                "--efi-boot-image",
                "--protective-msdos-label",
            ]);
        }
        xorriso.arg(&session.sysroot_dir);
        xorriso.arg("-o");
        xorriso.arg(&image);

        xorriso.log_command();
        xorriso.spawn()?.wait()?.exit_ok()?;

        if let Some(path) = limine_dir {
            let mut limine_deploy = Command::new(pathcat!(path, "limine-deploy"));
            limine_deploy.arg(&image);
            limine_deploy.log_command();
            limine_deploy.spawn()?.wait()?.exit_ok()?;
        }
    }

    Ok(())
}
