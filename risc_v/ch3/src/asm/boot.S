# boot.S
# bootloader for SoS
# Stephen Marz
# 8 February 2019

# Disable generation of compressed instructions.
.option norvc

# Define a .data section.
.section .data

# Define a .text.init section.
.section .text.init

# Execution starts here.
.global _start
_start:
	# Any hardware threads (hart) that are not bootstrapping
	# need to wait for an IPI
	csrr	t0, mhartid
	bnez	t0, 3f
	# SATP should be zero, but let's make sure
	csrw	satp, zero
	
	# Disable linker instruction relaxation for the `la` instruction below.
	# This disallows the assembler from assuming that `gp` is already initialized.
	# This causes the value stored in `gp` to be calculated from `pc`.
.option push
.option norelax
	la		gp, _global_pointer
.option pop
	# Set all bytes in the BSS section to zero.
	la 		a0, _bss_start
	la		a1, _bss_end
	bgeu	a0, a1, 2f
1:
	sd		zero, (a0)
	addi	a0, a0, 8
	bltu	a0, a1, 1b
2:
	# Control registers, set the stack, mstatus, mepc,
	# and mtvec to return to the main function.
	# li		t5, 0xffff;
	# csrw	medeleg, t5
	# csrw	mideleg, t5
	la		sp, _stack
	# Setting `mstatus` register:
	# 0b11 << 11: Machine's previous protection mode is 3 (MPP=3).
	li		t0, 0b11 << 11
	csrw	mstatus, t0
	# Machine's exception program counter (MEPC) is set to `kinit`.
	la		t1, kinit
	csrw	mepc, t1
	# Machine's trap vector base address is set to `asm_trap_vector`.
	la		t2, asm_trap_vector
	csrw	mtvec, t2
	# Set the return address to get us into supervisor mode
	la		ra, 2f
	# We use mret here so that the mstatus register is properly updated.
	mret
2:
	# Setting `sstatus` register:
	# 1 << 8    : Supervisor's previous protection mode is 1 (SPP=1 [Supervisor]).
	# 1 << 5    : Supervisor's previous interrupt-enable bit is 1 (SPIE=1 [Enabled]).
	# 1 << 1    : Supervisor's interrupt-enable bit is 1 (SIE=1 [Enabled]).
	li		t0, (0b1 << 8) | (1 << 5) | (1 << 1)
	csrw	sstatus, t0
	la		t1, kmain
	csrw	sepc, t1
	csrw	satp, a0
	sret
3:

	# Parked harts go here. We need to set these
	# to only awaken if it receives a software interrupt,
	# which we're going to call the SIPI (Software Intra-Processor Interrupt).
	# We only use these to run user-space programs, although this may
	# change.
4:
	wfi
	j		4b

