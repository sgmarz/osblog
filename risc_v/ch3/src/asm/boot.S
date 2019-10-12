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
	# The stack grows from bottom to top, so we put the stack pointer
	# to the very end of the stack range.
	la		sp, _stack_end
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
	# We set the return address (ra above) to this label. When kinit() is finished
	# in Rust, it will return here.

	# Setting `sstatus` (supervisor status) register:
	# 1 << 8    : Supervisor's previous protection mode is 1 (SPP=1 [Supervisor]).
	# 1 << 5    : Supervisor's previous interrupt-enable bit is 1 (SPIE=1 [Enabled]).
	# 1 << 1    : Supervisor's interrupt-enable bit will be set to 1 after sret.
	# We set the "previous" bits because the sret will write the current bits
	# with the previous bits.
	li		t0, (1 << 8) | (1 << 5)
	csrw	sstatus, t0
	la		t1, kmain
	csrw	sepc, t1
	# Setting `mideleg` (machine interrupt delegate) register:
	# 1 << 1   : Software interrupt delegated to supervisor mode
	# 1 << 5   : Timer interrupt delegated to supervisor mode
	# 1 << 9   : External interrupt delegated to supervisor mode
	# By default all traps (interrupts or exceptions) automatically
	# cause an elevation to the machine privilege mode (mode 3).
	# When we delegate, we're telling the CPU to only elevate to
	# the supervisor privilege mode (mode 1)
	li		t2, (1 << 1) | (1 << 5) | (1 << 9)
	csrw	mideleg, t2
	# Setting `sie` (supervisor interrupt enable) register:
	# This register takes the same bits as mideleg
	# 1 << 1    : Supervisor software interrupt enable (SSIE=1 [Enabled])
	# 1 << 5    : Supervisor timer interrupt enable (STIE=1 [Enabled])
	# 1 << 9    : Supervisor external interrupt enable (SEIE=1 [Enabled])
	csrw	sie, t2
	# Setting `stvec` (supervisor trap vector) register:
	# Essentially this is a function pointer, but the last two bits can be 00 or 01
	# 00        : All exceptions set pc to BASE
	# 01        : Asynchronous interrupts set pc to BASE + 4 x scause
	la		t3, asm_trap_vector
	csrw	stvec, t3
	# kinit() is required to return back the SATP value (including MODE) via a0
	csrw	satp, a0
	# Force the CPU to take our SATP register.
	# To be efficient, if the address space identifier (ASID) portion of SATP is already
	# in cache, it will just grab whatever's in cache. However, that means if we've updated
	# it in memory, it will be the old table. So, sfence.vma will ensure that the MMU always
	# grabs a fresh copy of the SATP register and associated tables.
	sfence.vma
	# sret will put us in supervisor mode and re-enable interrupts
	sret
3:

	# Parked harts go here. We need to set these
	# to only awaken if it receives a software interrupt,
	# which we're going to call the SIPI (Software Intra-Processor Interrupt).
	# We call the SIPI by writing the software interrupt into the Core Local Interruptor (CLINT)
	# Which is calculated by: base_address + hart * 4
	# where base address is 0x0200_0000 (MMIO CLINT base address)
	# We only use additional harts to run user-space programs, although this may
	# change.
4:
	wfi
	j		4b


