.pushsection .text
.option norelax



.altmacro

.macro _save_gp_reg x
        sd      x\x, (8*\x)(sp)
.endm
.macro _rstor_gp_reg x
        ld      x\x, (8*\x)(sp)
.endm

.macro save_gp_regs
        addi    sp, sp, -(8*32)
.set x, 0
.rept 32
        _save_gp_reg %x
.set x, x+1
.endr
.endm
.macro rstor_gp_regs
.set x, 0
.rept 32
        _rstor_gp_reg %x
.set x, x+1
.endr
        addi    sp, sp, 8*32
.endm

.section .text.trap_entry,"ax",@progbits
.global trap_entry
.type trap_entry,@function
.align 4
trap_entry:
        save_gp_regs

        addi    sp, sp, -32
        csrr    t0, sstatus
        sd      t0, 24(sp)
        csrr    t0, stval
        sd      t0, 16(sp)
        csrr    t0, sepc
        sd      t0, 8(sp)
        csrr    t0, scause
        sd      t0, 0(sp)

        mv      s1, sp

        andi    sp, sp, ~0xf
        mv      a0, s1
        call    rust_trap_entry

        mv      sp, s1
        addi    sp, sp, 24

        rstor_gp_regs
        sret

.size trap_entry, . - trap_entry

.popsection