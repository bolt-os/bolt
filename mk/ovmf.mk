
OVMF_VERSION?= latest

OVMF_DIR=	${DEPSDIR}/ovmf-${OVMF_VERSION}

OVMF_URL=	https://github.com/rust-osdev/ovmf-prebuilt/releases/latest/download
.if ${OVMF_VERSION} != "latest"
OVMF_URL=	https://github.com/rust-osdev/ovmf-prebuilt/releases/download/${OVMF_VERSION}
.endif

OVMF_CODE= ${OVMF_DIR}/OVMF_CODE.fd
OVMF_VARS= ${OVMF_DIR}/OVMF_VARS.fd

${OVMF_CODE}:
	curl -sLo ${OVMF_CODE} --create-dirs ${OVMF_URL}/OVMF_CODE-pure-efi.fd

${OVMF_VARS}:
	curl -sLo ${OVMF_VARS} --create-dirs ${OVMF_URL}/OVMF_VARS-pure-efi.fd
