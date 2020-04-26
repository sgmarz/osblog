
.section .text.init
.global _start
_start:
	call	main
	# Exit system call after main
	li	a0, 93
	j 	make_syscall
.type _start, function
.size _start, .-_start
