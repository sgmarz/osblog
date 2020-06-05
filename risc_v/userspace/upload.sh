#!/bin/sh

if [ $# -ne 1 ]; then
	echo "You provied $# parameters, need 1"
	exit 1
fi

if [ ! -r $1 ]; then
	echo "Unknown file $1"
	exit 2
fi

if [ $UID -ne 0 ]; then
	echo "You are not running as root, this might not work."
fi

losetup /dev/loop0 ../hdd.dsk
mount /dev/loop0 /mnt
cp $1 /mnt
umount /dev/loop0
losetup -d /dev/loop0
