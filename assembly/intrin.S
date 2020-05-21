.intel_syntax noprefix
.section .text
.global calc_asm
calc_asm:
	# rdi rsi rdx rcx
	movupd	xmm0, [rsi + 0]
	movupd	xmm1, [rsi + 16]
	movupd  xmm2, [rsi + 32]
	movupd	xmm3, [rsi + 48]

	movupd	xmm4, [rdx]

	mulps	xmm0, xmm4
	mulps	xmm1, xmm4
	mulps	xmm2, xmm4
	mulps	xmm3, xmm4

	haddps	xmm0, xmm0
	haddps	xmm0, xmm0
	haddps	xmm1, xmm1
	haddps	xmm1, xmm1
	haddps	xmm2, xmm2
	haddps	xmm2, xmm2
	haddps	xmm3, xmm3
	haddps	xmm3, xmm3

	movss	[rdi + 0], xmm0
	movss	[rdi + 4], xmm1
	movss	[rdi + 8], xmm2
	movss	[rdi + 12], xmm3


	ret
