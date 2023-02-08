#[cfg(target_arch = "x86_64")]
#[path = "../arch/x86_64/src/mod.rs"]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
