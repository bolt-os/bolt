
.if ${SPARK_PATH} == ""
SPARK_DIR=		${DEPSDIR}/spark
.else
SPARK_DIR=		${SPARK_PATH}
.endif
SPARK_URL=		https://github.com/bolt-os/spark.git
SPARK_CFG=		${KERN_ROOT}/conf/spark.cfg
SPARK_ELF=		${SPARK_DIR}/spark-out/spark-${TARGET:S/riscv/riscv64/}-sbi-${PROFILE}.elf
SPARK_BIN=		${SPARK_ELF:.elf=.bin}

${SPARK_DIR}:
.if ${SPARK_PATH} == ""
	git clone ${SPARK_URL} ${SPARK_DIR} --depth=1
.endif

_RELEASE=
.if ${PROFILE} == "release"
_RELEASE= --release
.endif

${SPARK_ELF}: .MAKE .PHONY ${SPARK_DIR}
	cd ${SPARK_DIR}; cargo xtask build --target riscv64-sbi ${_RELEASE}
