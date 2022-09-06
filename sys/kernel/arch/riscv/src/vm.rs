pub fn with_userspace_access<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_sstatus: usize;

    unsafe {
        asm!(
            "csrrc {}, sstatus, {}",
            out(reg) prev_sstatus,
            in(reg) 1usize << 18,
            options(nomem, nostack, preserves_flags),
        );
    }

    let result = f();

    if prev_sstatus & 1 << 18 != 0 {
        unsafe {
            asm!(
                "csrs sstatus, {}",
                in(reg) 1usize << 18,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    result
}
