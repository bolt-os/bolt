
OPENSBI_DIR=		${DEPSDIR}/opensbi
OPENSBI_URL=		https://github.com/riscv-software-src/opensbi/releases/latest/download

_OPENSBI_FW_PREFIX=	${OPENSBI_DIR}/opensbi/share/opensbi/lp64/generic/firmware/
OPENSBI_FW_DYNAMIC=	${_OPENSBI_FW_PREFIX}/fw_dynamic.bin
OPENSBI_FW_JUMP=	${_OPENSBI_FW_PREFIX}/fw_jump.bin
OPENSBI_FW_PAYLOAD=	${_OPENSBI_FW_PREFIX}/fw_payload.bin

${OPENSBI_FW_DYNAMIC} ${OPENSBI_FW_JUMP} ${OPENSBI_FW_PAYLOAD}: opensbi

opensbi: .PHONY .MAKE
	PDIR=${OPENSBI_DIR} ${BOLT_ROOT}/mk/opensbi.sh
