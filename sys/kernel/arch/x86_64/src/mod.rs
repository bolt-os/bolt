pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") };
    }
}

unsafe fn port3f8_write(s: &str) {
    asm!(
        "rep outsb",
        in("rsi") s.as_ptr(),
        in("ecx") s.len(),
        in("edx") 0x3f8,
        options(nostack, preserves_flags),
    );
}

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    port3f8_write("hello, world!\r\n");
    hcf();
}
