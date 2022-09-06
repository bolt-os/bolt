
LIMINE_DIR=		${DEPSDIR}/limine
LIMINE_URL= 	https://github.com/limine-bootloader/limine.git
LIMINE_CFG=		${KERN_ROOT}/conf/limine.cfg
LIMINE_FILES_=	BOOTX64.EFI limine.sys limine-cd.bin limine-cd-efi.bin
LIMINE_FILES=	${LIMINE_FILES_:S,^,${LIMINE_DIR}/,}

${LIMINE_DIR}:
	git clone ${LIMINE_URL} ${LIMINE_DIR} --branch=v3.0-branch-binary --depth=1

${LIMINE_DIR}/limine-deploy.c ${LIMINE_FILES}: ${LIMINE_DIR}

${LIMINE_DIR}/limine-deploy: ${LIMINE_DIR}/limine-deploy.c
	cd ${LIMINE_DIR}; ${GMAKE} CC='${CC}' limine-deploy
