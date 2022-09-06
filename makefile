
CARGO?=		cargo
NASM?=		nasm
XORRISO?=	xorriso

.if ${HOST_OS} == "Linux"
GMAKE?=		make
.else
GMAKE?=		gmake
.endif

CARGO!=		which ${CARGO}
GMAKE!=		which ${GMAKE}
NASM!=		which ${NASM}
XORRISO!=	which ${XORRISO}

OPTIONS_DEFAULT_VALUES=

OPTIONS_DEFAULT_VALUES+=		VERBOSE/no
DESCRIPTION_VERBOSE=			Be more verbose.

OPTIONS_DEFAULT_VALUES+=		RUN_ACCEL/yes
DESCRIPTION_RUN_ACCEL=			Enable hardware acceleration in QEMU.

OPTIONS_DEFAULT_VALUES+=		RUN_SMP/yes
DESCRIPTION_RUN_SMP=			Enable multiple processors when running with QEMU.

OPTIONS_DEFAULT_VALUES+=		RUN_EFI/yes
DESCRIPTION_RUN_EFI=			Use EFI firmware when running with QEMU.

OPTIONS_DEFAULT_VALUES+=		RUN_GFX/no
DESCRIPTION_RUN_EFI=			Enable graphics output when running with QEMU.

.include <compiler.mk>
.include <options.mk>
.-include "local.mk"

BOLT_ROOT= ${.CURDIR}
.export BOLT_ROOT

TARGET_DIR=				${BOLT_ROOT}/target
CARGO_TARGET_DIR=		${TARGET_DIR}
.export CARGO_TARGET_DIR

KERN_ROOT=				${BOLT_ROOT}/sys
KERN_ARCH_ROOT=			${KERN_ROOT}/kernel/arch

TARGET?= x86_64
PROFILE?= debug

BOLTK.TARGET_TRIPLE=	${TARGET:S/riscv/riscv64imac/}-unknown-none
BOLTK.RUST_TARGET=		${BOLTK.TARGET_TRIPLE}
.if ${TARGET} == "riscv"
BOLTK.RUST_TARGET=		${KERN_ARCH_ROOT}/conf/${BOLTK.TARGET_TRIPLE}.json
.endif
BOLTK.TARGET_DIR=		${TARGET_DIR}/${BOLTK.TARGET_TRIPLE}/${PROFILE}
BOLTK=					${BOLTK.TARGET_DIR}/boltk

TARGET.TRIPLE=		${TARGET:S/riscv/riscv64imac/}-unknown-none

TARGET_DIR=			${.CURDIR}/target
CARGO_TARGET_DIR=	${TARGET_DIR}
.export 			CARGO_TARGET_DIR

SYS_ROOT=			${.CURDIR}/sys
KERN_ROOT=			${SYS_ROOT}/kernel
KERN_ARCH_ROOT=		${KERN_ROOT}/arch/${TARGET}

BOLTK.RUST_TARGET=	${TARGET.TRIPLE}
BOLTK_OUTDIR=		${TARGET_DIR}/${BOLTK.RUST_TARGET}/${PROFILE}
BOLTK=				${BOLTK_OUTDIR}/boltk
.if make(cargo-test-runner)
BOLTK!=				cat ${KERN_ROOT}/.test-boltk
cargo-test-runner: .MAKE .PHONY run
.endif
DEPSDIR=		${TARGET_DIR}/bolt-deps

.MAIN: all

all: kernel

clean: .MAKE .PHONY
	rm -rf ${TARGET_DIR}

CARGO.RELEASE= ${PROFILE:Mrelease:S/r/--r/}

.if ${TARGET} == "x86_64"
BOLTK.TARGET=	x86_64-unknown-none
.elif ${TARGET} == "riscv"
BOLTK.TARGET=	${KERN_ARCH_ROOT}/conf/riscv64imac-unknown-none.json
.endif

${BOLTK}: .PHONY .MAKE kernel
kernel: .MAKE .PHONY
	cd ${SYS_ROOT};   ${CARGO} build ${CARGO.RELEASE} --target ${BOLTK.TARGET} --timings

# Other languages compiled in with `xcomp` emit their dependency files somewhere in KERN_OUTDIR.
# Since this directory can change between invocations of cargo, build.rs writes the current
# path to the file #{KERN_ROOT}/.outdir.
.if exists(${KERN_ROOT}/.outdir)
KERN_OUTDIR!= cat ${KERN_ROOT}/.outdir
.if exists(${KERN_OUTDIR})
.for depfile in ${:!find ${KERN_OUTDIR} -name '*.d'!}
.include "${depfile}"
.endfor
.endif
.endif

.include "mk/limine.mk"
.include "mk/spark.mk"
.include "mk/opensbi.mk"
.include "mk/ovmf.mk"

SYSROOT_DIR=	${TARGET_DIR}/boltk-sysroot
SYSROOT=		${SYSROOT_DIR}/bolt-${TARGET}

.include "mk/sysroot.mk"


#
# QEMU
#

QEMU?=			qemu-system-${TARGET:S/riscv/riscv64/}
QEMU_SOCKET?=	qemu-monitor.socket
QEMU_LOG?=		qemu-log.txt
QEMU.memory?=	2000

QEMU.deps=
QEMU.deps+=		sysroot

QEMUFLAGS=
QEMUFLAGS+=	-m ${QEMU.memory}
QEMUFLAGS+=	-no-reboot -no-shutdown
QEMUFLAGS+=	-D ${QEMU_LOG} -d int,guest_errors
QEMUFLAGS+=	-monitor unix:${QEMU_SOCKET},server,nowait

.if ${MK_RUN_GFX} != "no"
QEMUFLAGS+=	-serial stdio
.else
QEMUFLAGS+=	-nographic
.endif

.if ${MK_RUN_SMP} != "no"
QEMUFLAGS+= -smp 4
.endif

# ==================================================================
#	Target Specifics

.if ${TARGET} == "x86_64"
QEMU.machine=		q35
QEMU.cpu=			qemu64
QEMU.cpu_features=	+smap,+smep

QEMUFLAGS+=			-M smm=off
.if ${MK_RUN_EFI} == "yes"
QEMUFLAGS+=			-drive if=pflash,format=raw,file=${OVMF_CODE},readonly=on
QEMUFLAGS+=			-drive if=pflash,format=raw,file=${OVMF_VARS}
QEMU.deps+=			${OVMF_CODE} ${OVMF_VARS}
.endif
QEMUFLAGS+= 		-cdrom ${SYSROOT}.iso
QEMU.deps+=			sysroot

.elif ${TARGET} == "riscv"
QEMU.machine=		virt
QEMU.cpu=			rv64
QEMU.cpu_features=

QEMU.deps+=			${SPARK_ELF} ${BOLTK} sysroot
QEMUFLAGS+= 		-kernel ${SPARK_ELF}
QEMUFLAGS+= 		-fw_cfg opt/org.spark/kernel,file=${BOLTK}
QEMUFLAGS+=			-fw_cfg opt/org.spark/config,file=${KERN_ROOT}/conf/spark.cfg
# Pass the spark binary itself so we can get nicer backtraces.
QEMUFLAGS+=			-fw_cfg opt/org.spark/self,file=${SPARK_ELF}
QEMUFLAGS+=			-device ramfb

.endif

QEMUFLAGS+=			-device nvme,serial=deadbeef,drive=nvm
QEMUFLAGS+=			-drive format=raw,id=nvm,file=${SYSROOT}.iso,if=none

QEMUFLAGS+=			-device virtio-blk,drive=hd0
QEMUFLAGS+=			-drive format=raw,id=hd0,file=${SYSROOT}.iso,if=none

# ==================================================================
#	Hardware Acceleration

QEMU.accel=			none
.if ${TARGET} == ${HOST_MACHINE} && ${MK_RUN_ACCEL:Uno} == "yes"

.if ${HOST_OS} == "Darwin"
.if ${:!sysctl -n kern.hv_support!} == "1"
# `-cpu host` just does not want to work :^(
# QEMU.cpu=	host
QEMU.accel=	hvf
.endif

.elif ${HOST_OS} == "Linux"
.if exists(/dev/kvm)
QEMU.cpu=	host
QEMU.accel=	kvm
.endif

.endif
.endif

# ==================================================================

QEMUFLAGS+=	-machine ${QEMU.machine}
.if ${QEMU.cpu_features} != ""
QEMUFLAGS+= -cpu ${QEMU.cpu},${QEMU.cpu_features}
.else
QEMUFLAGS+= -cpu ${QEMU.cpu}
.endif
.if ${QEMU.accel:Unone} != "none"
QEMUFLAGS+= -accel ${QEMU.accel}
.endif

run: .PHONY ${QEMU.deps}
	${QEMU} ${QEMUFLAGS}

monitor: .PHONY
	socat -,echo=0,icanon=0 unix-connect:${QEMU_SOCKET}
