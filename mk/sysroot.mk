
SYSROOT_DIR=	${TARGET_DIR}/bolt-sysroot
SYSROOT=		${SYSROOT_DIR}/bolt-${TARGET}-${PROFILE}
SYSROOT_ISO=	${SYSROOT}.iso


sysroot: .MAKE .PHONY ${SYSROOT_ISO}


SYSROOT.deps=	${BOLTK}

.if ${TARGET} == "x86_64"
SYSROOT.deps+=	${LIMINE_CFG} ${LIMINE_FILES}
.elif ${TARGET} == "riscv"
SYSROOT.deps+=	${SPARK}
.endif

# Copy everything into the sysroot directory
${SYSROOT}: ${SYSROOT.deps}
	rm -rf ${SYSROOT}
	mkdir -p ${SYSROOT}/boot/EFI/BOOT
.if ${TARGET} == "x86_64"
	cp ${LIMINE_DIR}/BOOTX64.EFI		${SYSROOT}/boot/EFI/BOOT/
	cp ${LIMINE_FILES:N*BOOTX64.EFI}	${SYSROOT}/boot/
	cp ${KERN_ROOT}/conf/limine.cfg		${SYSROOT}/boot/
.endif
.if ${TARGET} == "riscv"
	cp ${KERN_ROOT}/conf/spark.cfg		${SYSROOT}/boot
.endif
	cp ${BOLTK}							${SYSROOT}/boot/boltk


SYSROOT.XORRISO_FLAGS=
SYSROOT.XORRISO_FLAGS+=	-as mkisofs
.if ${TARGET} == "x86_64"
SYSROOT.XORRISO_FLAGS+=	-b boot/limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table
SYSROOT.XORRISO_FLAGS+=	--efi-boot boot/limine-cd-efi.bin -efi-boot-part --efi-boot-image --protective-msdos-label
.endif

SYSROOT_ISO.deps= ${SYSROOT}

.if ${TARGET} == "x86_64"
SYSROOT_ISO.deps+= ${LIMINE_DIR}/limine-deploy
.endif

# Create a disk image from the root directory.
${SYSROOT_ISO}: ${SYSROOT_ISO.deps}
	rm -rf ${SYSROOT}.iso
	${XORRISO} ${SYSROOT.XORRISO_FLAGS} ${SYSROOT} -o ${SYSROOT}.iso
.if ${TARGET} == "x86_64"
	${LIMINE_DIR}/limine-deploy ${SYSROOT}.iso
.endif
