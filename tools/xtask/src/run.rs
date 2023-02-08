use crate::{
    build::{self},
    pathcat, CommandExt, Session, XtaskArgs,
};
use std::{borrow::Cow, ffi::OsString, path::PathBuf, process::Command};

#[derive(Clone, Debug, clap::Args)]
pub struct RunArgs {
    #[clap(flatten)]
    pub build_args: build::BuildArgs,

    #[arg(long)]
    pub log: Option<Option<PathBuf>>,

    #[arg(long)]
    pub no_accel: bool,

    #[arg(last = true)]
    pub emulator_args: Vec<OsString>,
}

impl RunArgs {
    pub fn xtask_args(&self) -> &XtaskArgs {
        self.build_args.xtask_args()
    }
}

pub fn main(session: &mut Session, args: &mut RunArgs) -> anyhow::Result<()> {
    build::main(session, &mut args.build_args)?;
    
    let image = pathcat!(
        session.output_dir,
        format!("{}.iso", args.xtask_args().build_name())
    );
    println!("running {}", image.display());

    let mut qemu = Command::new(format!("qemu-system-{}", session.target.name));

    match session.target.name {
        "x86_64" => {
            qemu.args(["-machine", "q35", "-cpu", "qemu64,+smep,+smap"]);
        }
        _ => todo!(),
    }

    qemu.args(["-no-reboot", "-no-shutdown", "-serial", "mon:stdio"]);

    if let Some(log_file) = args.log.as_ref() {
        let log_file = if let Some(path) = log_file.as_ref() {
            Cow::Borrowed(path)
        } else {
            Cow::Owned(pathcat!(session.output_dir, "qemu-log.txt"))
        };
        qemu.arg("-D");
        qemu.arg(&*log_file);
    }

    qemu.arg("-cdrom");
    qemu.arg(image);

    qemu.log_command();
    qemu.spawn()?.wait()?.exit_ok()?;

    Ok(())
}
