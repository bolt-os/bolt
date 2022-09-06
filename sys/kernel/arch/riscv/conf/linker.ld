/*
 * Copyright (c) xvanc and contributors
 * SPDX-License-Identifier: BSD-2-Clause
 */

ENTRY(_start)
OUTPUT_ARCH(riscv64)
OUTPUT_FORMAT(elf64-littleriscv)

KERN_BASE  = 0xffffffff80000000;

SECTIONS
{
    . = KERN_BASE + SIZEOF_HEADERS;

    PROVIDE(__executable_start = .);

    .hash                   : { *(.hash) }
    .gnu.hash               : { *(.gnu.hash) }

    .dynsym                 : { *(.dynsym) }
    .dynstr                 : { *(.dynstr) }

    .rela.dyn               :
    {
        *(.rela.dyn)
        *(.rela.text    .rela.text.*)
        *(.rela.rodata  .rela.rodata.*)
        *(.rela.data.rel.ro*)
        *(.rela.got)
        *(.rela.data    .rela.data.*)
        *(.rela.tdata   .rela.tdata.*)
        *(.rela.tbss    .rela.tbss.*)
        *(.rela.bss     .rela.bss.*)
        *(.rela.ifunc)
    }
    .rela.plt               : { *(.rela.plt) *(.rela.iplt) }

    . += CONSTANT(MAXPAGESIZE);

    .plt                    : { *(.plt) *(.iplt) }
    .text                   : { *(.text .text.*) }
    PROVIDE(__etext = .);

    . += CONSTANT(MAXPAGESIZE);

    .rodata                 : { *(.rodata .rodata.*) }

    .note.gnu.build-id      :
    {
        PROVIDE(__build_id = .);
        KEEP(*(.note.gnu.build-id))
    }

    .eh_frame_hdr           :
    {
        PROVIDE(__eh_frame_hdr = .);
        *(.eh_frame_hdr)
    }

    . += CONSTANT(MAXPAGESIZE);

    PROVIDE(__eh_frame = .);
    .eh_frame               : { KEEP(*(.eh_frame)) }
    .gcc_except_table       : { KEEP(*(.gcc_except_table)) }

    . += CONSTANT(MAXPAGESIZE);

    .tdata                  : { *(.tdata .tdata.*) }
    .tbss                   : { *(.tbss .tbss.*) }

    .data.rel.ro            :
    {
        *(.data.rel.ro.local .data.rel.ro.local.*)
        *(.data.rel.ro .data.rel.ro.*)
    }

    .dynamic                : { *(.dynamic) }


    . += CONSTANT(MAXPAGESIZE);
    . = DATA_SEGMENT_RELRO_END(0, .);

    .got                    : { *(.got) *(.igot) }
    .got.plt                : { *(.got.plt) *(.igot.plt) }

        . += CONSTANT(MAXPAGESIZE);
    .data                   : { *(.data .data.*) }
    .sdata                  : { *(.sdata .sdata.*) }
    PROVIDE(__global_pointer$ = .);
    .sbss                   : { *(.sbss .sbss.*) }
    .bss                    : { *(.dynbss) *(.bss .bss.*) }

    . = DATA_SEGMENT_END(.);

    .comment              0 : { *(.comment) }
    .debug                0 : { *(.debug) }
    .debug_abbrev         0 : { *(.debug_abbrev) }
    .debug_aranges        0 : { *(.debug_aranges) }
    .debug_frame          0 : { *(.debug_frame) }
    .debug_funcnames      0 : { *(.debug_funcnames) }
    .debug_info           0 : { *(.debug_info .gnu.linkonce.wi.*) }
    .debug_line           0 : { *(.debug_line) }
    .debug_loc            0 : { *(.debug_loc) }
    .debug_macinfo        0 : { *(.debug_macinfo) }
    .debug_pubnames       0 : { *(.debug_pubnames) }
    .debug_pubtypes       0 : { *(.debug_pubtypes) }
    .debug_ranges         0 : { *(.debug_ranges) }
    .debug_sfnames        0 : { *(.debug_sfnames) }
    .debug_srcinfo        0 : { *(.debug_srcinfo) }
    .debug_str            0 : { *(.debug_str) }
    .debug_typenames      0 : { *(.debug_typenames) }
    .debug_varnames       0 : { *(.debug_varnames) }
    .debug_weaknames      0 : { *(.debug_weaknames) }
    .line                 0 : { *(.line) }
    .shstrtab             0 : { *(.shstrtab) }
    .strtab               0 : { *(.strtab) }
    .symtab               0 : { *(.symtab) }
}
