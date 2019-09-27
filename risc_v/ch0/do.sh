#!/bin/bash
# For building cross compilers
# Use this at your own risk!
# I make no warranties or guarantees with this script!
# Stephen Marz
# 15 Jan 2018

. ./.build.config
if [ $# -eq 0 ]; then
	echo "Must provide a number"
	echo "0 - Binutils"
	echo "1 - GCC Stage 1"
	echo "2 - Linux Headers"
	echo "3 - GLIBC Headers"
	echo "4 - GLIBC"
	echo "5 - GCC Stage 2"
	echo "6 - QEMU"
	echo "7 - Libs and Links"
	echo "Add 90 if you just want to build that one stage"
	echo "99 - Clean"
	exit 99
else
	ARG=$1
fi
#Build BINUTILS
if [ $ARG -le 0 -o $ARG -eq 90 ]; then
	echo "+-+-+-+ BINUTILS +-+-+-+"
	mkdir -p ${BUILD_BINUTILS}
	cd ${BUILD_BINUTILS}
	${BUILD_ROOT}/binutils-gdb/configure --target=${TARGET} --prefix=${PREFIX} --with-sysroot=${SYSROOT} --disable-multilib --disable-werror --disable-nls --with-expat=yes --enable-gdb > ${BUILD_ROOT}/binutils.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring BINUTILS"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/binutils.log
		exit 1
	fi
	cd ${BUILD_ROOT}
	make -C ${BUILD_BINUTILS} -j${JOBS} >> ${BUILD_ROOT}/binutils.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error building BINUTILS"
		echo "~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/binutils.log
		exit 1
	fi
	${USE_SUDO} make -C ${BUILD_BINUTILS} install >> ${BUILD_ROOT}/binutils.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing BINUTILS"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/binutils.log
		exit 1
	fi
fi

#Build GCC Stage 1
if [ $ARG -le 1 -o $ARG -eq 91 ]; then
	echo "+-+-+-+ GCC STAGE 1 +-+-+-+"
	sed -i "s|\"/lib/ld-linux-${ARCH}|\"${SYSROOT}/lib/ld-linux-${ARCH}|" ${BUILD_ROOT}/gcc/gcc/config/${ARCH}/${LIB_HEADER}
	mkdir -p ${BUILD_GCC_S1}
	cd ${BUILD_GCC_S1}
	${BUILD_ROOT}/gcc/configure --target=${TARGET} --prefix=${PREFIX} --with-sysroot=${SYSROOT} --with-newlib --without-headers --disable-shared --disable-threads --with-system-zlib --enable-tls --enable-languages=c --disable-libatomic --disable-libmudflap --disable-libssp --disable-libquadmath --disable-libgomp --disable-nls --disable-bootstrap --enable-checking=yes --disable-multilib --with-abi=${ABI} --with-arch=${ISA} > ${BUILD_ROOT}/gccs1.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring GCC stage 1"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs1.log
		exit 2
	fi
	cd ${BUILD_ROOT}
	make -j${JOBS} -C ${BUILD_GCC_S1} >> ${BUILD_ROOT}/gccs1.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error building GCC stage 1"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs1.log
		exit 2
	fi
	${USE_SUDO} make -C ${BUILD_GCC_S1} install >> ${BUILD_ROOT}/gccs1.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing GCC stage 1"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs1.log
		exit 2
	fi
fi

#Build Linux Headers
if [ $ARG -le 2 -o $ARG -eq 92 ]; then
	echo "+-+-+-+ LINUX HEADERS +-+-+-+"
	if [ ! -x ${BUILD_ROOT}/linux-${LINUX_VER} ]; then
		tar xf ${BUILD_ROOT}/linux-${LINUX_VER}.tar.xz -C ${BUILD_ROOT} > ${BUILD_ROOT}/linhdr.log 2>&1
	fi
	if [ $? -ne 0 ]; then
		echo "Error unpacking Linux Headers"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/linhdr.log
		exit 3
	fi
	make ARCH=${BUILD_LINUX_ARCH} INSTALL_HDR_PATH=${BUILD_LINUX_HEADERS} -C ${BUILD_LINUX} defconfig >> ${BUILD_ROOT}/linhdr.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring Linux Headers"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/linhdr.log
		exit 3
	fi
	make ARCH=${BUILD_LINUX_ARCH} INSTALL_HDR_PATH=${BUILD_LINUX_HEADERS} -C ${BUILD_LINUX} headers_install >> ${BUILD_ROOT}/linhdr.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing Linux Headers"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/linhdr.log
		exit 3
	fi
fi
if [ $ARG -le 3 -o $ARG -eq 93 ]; then
	#Build GLIBC Headers
	echo "+-+-+-+ GLIBC HEADERS +-+-+-+"
	mkdir -p ${BUILD_GLIBC_S1}
	cd ${BUILD_GLIBC_S1}
	${BUILD_ROOT}/glibc/configure --host=${TARGET} --prefix=${SYSROOT}/usr --enable-shared --with-headers=${BUILD_LINUX_HEADERS}/include --disable-multilib --enable-kernel=3.0.0 > ${BUILD_ROOT}/glibchdr.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring GLIBC headers"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibchdr.log
		exit 4
	fi
	cd ${BUILD_ROOT}
	${USE_SUDO} make -C ${BUILD_GLIBC_S1} install-headers >> ${BUILD_ROOT}/glibchdr.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing GLIBC headers"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibchdr.log
		exit 4
	fi
	${USE_SUDO} cp -a ${BUILD_LINUX_HEADERS}/include/* ${SYSROOT}/usr/include/ >> ${BUILD_ROOT}/glibchdr.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error copying include files"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibchdr.log
		exit 4
	fi
fi
if [ $ARG -le 4 -o $ARG -eq 94 ]; then
	#Build GLIBC
	echo "+-+-+-+ GLIBC +-+-+-+"
	mkdir -p ${BUILD_GLIBC_S2}
	cd ${BUILD_GLIBC_S2}
	${BUILD_ROOT}/glibc/configure --host=${TARGET} --prefix=/usr --disable-werror --enable-tls --disable-nls --enable-shared --enable-obsolete-rpc --with-headers=${SYSROOT}/usr/include --disable-multilib --enable-kernel=3.0.0 > ${BUILD_ROOT}/glibc.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring GLIBC"
		echo "~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibc.log
		exit 5
	fi
	cd ${BUILD_ROOT}
	make -C ${BUILD_GLIBC_S2} -j${JOBS} >> ${BUILD_ROOT}/glibc.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error building GLIBC"
		echo "~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibc.log
		exit 5
	fi
	${USE_SUDO} make -C ${BUILD_GLIBC_S2} install install_root=${SYSROOT} >> ${BUILD_ROOT}/glibc.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing GLIBC"
		echo "~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/glibc.log
		exit 5
	fi
	${USE_SUDO} ln -s ${SYSROOT}/lib64 ${SYSROOT}/lib
fi

if [ $ARG -le 5 -o $ARG -eq 95 ]; then
	#Build GCC Stage 2
	echo "+-+-+-+ GCC STAGE 2 +-+-+-+"
	mkdir -p ${BUILD_GCC_S2}
	cd ${BUILD_GCC_S2}
	${BUILD_ROOT}/gcc/configure --target=${TARGET} --prefix=${PREFIX} --with-sysroot=${SYSROOT} --with-system-zlib --enable-shared --enable-tls --enable-languages=c,c++ --disable-libmudflap --disable-libssp --disable-libquadmath --disable-nls --disable-bootstrap --disable-multilib --enable-checking=yes --with-abi=${ABI} > ${BUILD_ROOT}/gccs2.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring GCC stage 2"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs2.log
		exit 6
	fi
	cd ${BUILD_ROOT}
	make -C ${BUILD_GCC_S2} -j${JOBS} >> ${BUILD_ROOT}/gccs2.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error building GCC stage 2"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs2.log
		exit 6
	fi
	${USE_SUDO} make -C ${BUILD_GCC_S2} install >> ${BUILD_ROOT}/gccs2.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing GCC stage 2"
		echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/gccs2.log
		exit 6
	fi
	${USE_SUDO} cp -a ${PREFIX}/${TARGET}/lib* ${SYSROOT}
	if [ $? -ne 0 ]; then
		echo "Error copying libraries"
		echo "~~~~~~~~~~~~~~~~~~~~~~~"
		exit 6
	fi
fi

if [ $ARG -le 6 -o $ARG -eq 96 ]; then
	#Build QEMU
	echo "+-+-+-+ QEMU +-+-+-+"
	mkdir -p ${BUILD_QEMU}
	cd ${BUILD_QEMU}
	${BUILD_ROOT}/qemu/configure --prefix=${PREFIX} --interp-prefix=${SYSROOT} --target-list=riscv32-linux-user,riscv32-softmmu,${ARCH}${BITS}-linux-user,${ARCH}${BITS}-softmmu --enable-jemalloc --disable-werror > ${BUILD_ROOT}/qemu.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error configuring QEMU"
		echo "~~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/qemu.log
		exit 7
	fi
	cd ${BUILD_ROOT}
	make -C ${BUILD_QEMU} -j${JOBS} >> ${BUILD_ROOT}/qemu.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error building QEMU"
		echo "~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/qemu.log
		exit 7
	fi
	${USE_SUDO} make -C ${BUILD_QEMU} install >> ${BUILD_ROOT}/qemu.log 2>&1
	if [ $? -ne 0 ]; then
		echo "Error installing QEMU"
		echo "~~~~~~~~~~~~~~~~~~~~~"
		cat ${BUILD_ROOT}/qemu.log
		exit 7
	fi
fi

if [ $ARG -le 7 -o $ARG -eq 97 ]; then
	#Make Symlinks
	echo "+-+-+-+ SYMLINKS +-+-+-+"
	${USE_SUDO} ln -s ${PREFIX}/bin/${TARGET}-gcc ${PREFIX}/bin/${ARCH}${BITS}-gcc
	${USE_SUDO} ln -s ${PREFIX}/bin/${TARGET}-g++ ${PREFIX}/bin/${ARCH}${BITS}-g++
	${USE_SUDO} ln -s ${PREFIX}/bin/${TARGET}-objdump ${PREFIX}/bin/${ARCH}${BITS}-objdump
	${USE_SUDO} ln -s ${PREFIX}/bin/${TARGET}-gdb ${PREFIX}/bin/${ARCH}${BITS}-gdb

#Copy Libraries
echo "+-+-+-+ COPY LIBRARIES +-+-+-+"
${USE_SUDO} cp -a ${SYSROOT}/lib/* ${SYSROOT}/usr/lib${BITS}/${ABI}/
fi

if [ $ARG -eq 99 ]; then
	echo "+-+-+-+ CLEANING +-+-+-+"
	${USE_SUDO} rm -fr ${BUILD_BINUTILS}
	${USE_SUDO} rm -fr ${BUILD_GCC_S1}
	${USE_SUDO} rm -fr ${BUILD_LINUX}
	${USE_SUDO} rm -fr ${BUILD_GLIBC_S1}
	${USE_SUDO} rm -fr ${BUILD_GLIBC_S2}
	${USE_SUDO} rm -fr ${BUILD_GCC_S2}
	${USE_SUDO} rm -fr ${BUILD_QEMU}
	rm -fr *.log
fi
echo "+-+-+-+ !! DONE !! +-+-+-+"
