
.section .text
.global _start
_start:
.option push
.option norelax
	la	gp, __global_pointer$
.option pop
	call	main
	# Exit system call after main
	li	a0, 93
	j 	make_syscall
.type _start, function
.size _start, .-_start
