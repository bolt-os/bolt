
#![feature(
    exit_status_error,      // https://github.com/rust-lang/rust/issues/84908
    is_some_and,            // https://github.com/rust-lang/rust/issues/93050
)]
#![warn(clippy::all)]

use anyhow::{anyhow, Context};
use clap::{CommandFactory, Parser};
use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

mod build;
mod run;

#[inline(always)]
pub fn symlink<P, Q>(original: P, link: Q) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    _symlink(original.as_ref(), link.as_ref())
}

pub fn _symlink(original: &Path, link: &Path) -> anyhow::Result<()> {
    let target = if link.is_dir() && original.is_file() {
        let mut path = link.to_path_buf();
        path.push(original.components().last().unwrap());
        Cow::Owned(path)
    } else {
        Cow::Borrowed(link)
    };

    let error_context = || {
        anyhow!(
            "failed to create symlink: {} -> {} ({})",
            original.display(),
            link.display(),
            target.display()
        )
    };

    if target.exists() {
        if target.is_dir() {
            fs::remove_dir(&target).with_context(error_context)?;
        } else {
            fs::remove_file(&target).with_context(error_context)?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(original, &target).with_context(error_context)?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        if original.is_dir() {
            symlink_dir(original, &target).with_context(error_context)?;
        } else {
            symlink_file(original, &target).with_context(error_context)?;
        }
    }

    #[cfg(not(any(unix, windows)))]
    compile_error!("unsupported host platform");

    Ok(())
}

pub trait CommandExt {
    fn log_command(&self);
}

impl CommandExt for Command {
    fn log_command(&self) {
        println!(
            "% {} {}",
            self.get_program().to_str().unwrap(),
            self.get_args()
                .collect::<Vec<&OsStr>>()
                .join(OsStr::new(" "))
                .to_str()
                .unwrap(),
        );
    }
}

#[derive(Debug)]
pub struct BuildTarget {
    name: &'static str,
    // If the string ends with `.json` it will be prepended with `./target-specs/
    kernel_rust_target: &'static str,
    user_rust_target: &'static str,
}

impl BuildTarget {
    pub fn kernel_rust_target_name(&self) -> &str {
        self.kernel_rust_target.trim_end_matches(".json")
    }
    pub fn user_rust_target_name(&self) -> &str {
        self.user_rust_target.trim_end_matches(".json")
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
#[clap(rename_all = "snake_case")]
enum TargetArch {
    x86_64 = 0,
    riscv64,
}

static TARGETS: [BuildTarget; 2] = [
    BuildTarget {
        name: "x86_64",
        kernel_rust_target: "x86_64-unknown-none",
        user_rust_target: "x86_64-unknown-bolt.json",
    },
    BuildTarget {
        name: "riscv",
        kernel_rust_target: "riscv64imac-unknown-none.json",
        user_rust_target: "riscv64gc-unknown-bolt.json",
    },
];

#[derive(Clone, Parser, Debug)]
pub struct XtaskArgs {
    /// Which target to build for
    #[clap(long, default_value = "x86_64")]
    target: TargetArch,

    /// Create a release build
    #[clap(long)]
    release: bool,
}

impl XtaskArgs {
    pub fn build_name(&self) -> String {
        format!("bolt-{:?}-{}", self.target, self.profile())
    }

    pub fn profile(&self) -> &'static str {
        if self.release {
            "release"
        } else {
            "debug"
        }
    }
}

pub struct Session {
    target: &'static BuildTarget,
    pwd: PathBuf,
    /// `<pwd>/build`
    build_dir: PathBuf,
    /// `<build_dir>/target`
    cargo_target_dir: PathBuf,
    /// `<build_dir>/target/<kernel_rust_target>/<profile>`
    kernel_target_dir: PathBuf,
    // /// `<build_dir>/target/<user_rust_target>/<profile>`
    // user_target_dir: PathBuf,
    /// `<build_dir>/<target.name>-<profile>`
    output_dir: PathBuf,
    /// `<build_dir>/<target.name>-<profile>/root`
    sysroot_dir: PathBuf,
    /// `<pwd>/sys/kernel/arch/<target.name>`
    sys_arch_dir: PathBuf,
}

impl Session {
    pub fn cargo(&self) -> Command {
        Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
    }

    pub fn kernel_rust_target(&self) -> Cow<Path> {
        let kernel_target = self.target.kernel_rust_target;
        if kernel_target.ends_with(".json") {
            Cow::Owned(pathcat!(self.pwd, "target-specs", kernel_target))
        } else {
            Cow::Borrowed(Path::new(self.target.kernel_rust_target))
        }
    }
}

#[macro_export]
macro_rules! pathcat {
    ($($component:expr),*$(,)?) => {{
        let mut path = ::std::path::PathBuf::new();
        $(path.push(&$component);)*
        path
    }};
}

#[derive(Clone, clap::Parser)]
enum Cmdline {
    /// Build elements of the distribution
    Build(build::BuildArgs),
    /// Run an OS image in an emulator
    Run(run::RunArgs),
    #[clap(id = "self")]
    This(SelfArgs),
}

impl Cmdline {
    pub fn xtask_args(&self) -> &XtaskArgs {
        match self {
            Self::Build(args) => args.xtask_args(),
            Self::Run(args) => args.xtask_args(),
            Self::This(_) => unimplemented!(),
        }
    }
}

#[derive(Clone, Parser)]
pub struct SelfArgs {
    #[arg(long)]
    make_man_page: bool,
}

fn main() -> anyhow::Result<()> {
    let cmdline = Cmdline::parse();

    let pwd = env::current_dir()?;

    let build_dir = pathcat!(pwd, "build");
    let cargo_target_dir = pathcat!(build_dir, "target");

    if let Cmdline::This(ref args) = cmdline {
        if args.make_man_page {
            let man_dir = pathcat!(build_dir, "man");
            fs::create_dir_all(&man_dir)?;

            let mangen = clap_mangen::Man::new(<Cmdline as CommandFactory>::command());
            let mut file = File::create(pathcat!(man_dir, "bolt-xtask.1"))?;
            mangen.render(&mut file)?;

            let mangen = clap_mangen::Man::new(<build::BuildArgs as CommandFactory>::command());
            let mut file = File::create(pathcat!(man_dir, "bolt-xtask-build.1"))?;
            mangen.render(&mut file)?;
        }

        return Ok(());
    }

    let args = cmdline.xtask_args();
    let target = &TARGETS[args.target as usize];

    let kernel_target_dir = pathcat!(
        cargo_target_dir,
        target.kernel_rust_target_name(),
        args.profile()
    );
    // let user_target_dir = pathcat!(
    //     cargo_target_dir,
    //     target.user_rust_target_name(),
    //     args.profile()
    // );

    let output_dir = pathcat!(
        build_dir,
        format!("bolt-{}-{}", target.name, args.profile())
    );
    let sysroot_dir = pathcat!(output_dir, "root");
    let sys_arch_dir = pathcat!(pwd, "sys", "kernel", "arch", target.name);

    let mut session = Session {
        pwd,
        build_dir,
        cargo_target_dir,
        kernel_target_dir,
        // user_target_dir,
        output_dir,
        sysroot_dir,
        target,
        sys_arch_dir,
    };

    match cmdline {
        Cmdline::This(_) => unimplemented!(),
        Cmdline::Build(mut args) => build::main(&mut session, &mut args)?,
        Cmdline::Run(mut args) => run::main(&mut session, &mut args)?,
    }

    Ok(())
}
