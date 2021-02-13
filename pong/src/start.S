.section .text.init
.global _start
_start:
.option push
.option norelax
	la	gp, __global_pointer$
.option pop
	call	rustmain
	# Exit system call after main
	li	a7, 93
	ecall
.type _start, function
.size _start, .-_start
