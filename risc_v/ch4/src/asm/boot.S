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

	# Disable linker instruction relaxation for the `la` instruction below.
	# This disallows the assembler from assuming that `gp` is already initialized.
	# This causes the value stored in `gp` to be calculated from `pc`.
.option push
.option norelax
	la		gp, _global_pointer
.option pop
	csrw	satp, zero
	# Any hardware threads (hart) that are not bootstrapping
	# need to wait for an IPI
	csrr	t0, mhartid
	bnez	t0, 3f
	# SATP should be zero, but let's make sure

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
	# 0b01 << 11: Machine's previous protection mode is 2 (MPP=2).
	li		t0, 0b11 << 11
	csrw	mstatus, t0
	# li		t2, (1 << 1) | (1 << 5) | (1 << 9)
	li		t2, 0xffff
	csrw	mideleg, t2
	csrw	medeleg, t2
	# Do not allow interrupts while running kinit
	csrw	mie, zero
	# Machine's exception program counter (MEPC) is set to `kinit`.
	la		t1, kinit
	csrw	mepc, t1
	# Set the return address to get us into supervisor mode
	la		ra, 2f
	# We use mret here so that the mstatus register is properly updated.
	mret
2:
	# We set the return address (ra above) to this label. When kinit() is finished
	# in Rust, it will return here.

	# Setting `mstatus` (supervisor status) register:
	# 0b01 << 11 : Previous protection mode is 1 (MPP=01 [Supervisor]).
	# 1 << 7     : Previous machine interrupt-enable bit is 1 (MPIE=1 [Enabled])
	# 1 << 5     : Previous interrupt-enable bit is 1 (SPIE=1 [Enabled]).
	# We set the "previous" bits because the sret will write the current bits
	# with the previous bits.
	li		t0, (0b01 << 11) | (1 << 7) | (1 << 5)
	csrw	mstatus, t0
	# Setting `mideleg` (machine interrupt delegate) register:
	# 1 << 1   : Software interrupt delegated to supervisor mode
	# 1 << 5   : Timer interrupt delegated to supervisor mode
	# 1 << 9   : External interrupt delegated to supervisor mode
	# By default all traps (interrupts or exceptions) automatically
	# cause an elevation to the machine privilege mode (mode 3).
	# When we delegate, we're telling the CPU to only elevate to
	# the supervisor privilege mode (mode 1)
	# Setting `sie` (supervisor interrupt enable) register:
	# This register takes the same bits as mideleg
	# 1 << 1    : Supervisor software interrupt enable (SSIE=1 [Enabled])
	# 1 << 5    : Supervisor timer interrupt enable (STIE=1 [Enabled])
	# 1 << 9    : Supervisor external interrupt enable (SEIE=1 [Enabled])
	# 0xaaa = MEIP/SEIP and MTIP/STIP and MSIP/SSIP
	# Machine's trap vector base address is set to `m_trap_vector`, for
	# "machine" trap vector.
	la		t2, m_trap_vector
	csrw	mtvec, t2
	# Setting `stvec` (supervisor trap vector) register:
	# Essentially this is a function pointer, but the last two bits can be 00 or 01
	# 00        : All exceptions set pc to BASE
	# 01        : Asynchronous interrupts set pc to BASE + 4 x scause
	la		t3, s_trap_vector
	csrw	stvec, t3
	# Jump to kmain. We put the MPP = 01 for supervisor mode, so after
	# mret, we will jump to kmain in supervisor mode.
	la		t1, kmain
	csrw	mepc, t1
	li		t2, 0xaaa
	csrw	mie, t2
	csrw	sie, t2
	csrr	a0, mhartid
	mret
3:

	# Parked harts go here. We need to set these
	# to only awaken if it receives a software interrupt,
	# which we're going to call the SIPI (Software Intra-Processor Interrupt).
	# We call the SIPI by writing the software interrupt into the Core Local Interruptor (CLINT)
	# Which is calculated by: base_address + hart * 4
	# where base address is 0x0200_0000 (MMIO CLINT base address)
	# We only use additional harts to run user-space programs, although this may
	# change.

	la		sp, _stack_end
	li		t0, 0x10000
	csrr	a0, mhartid
	mul		t0, t0, a0
	sub		sp, sp, t0

	li		t0, 0b11 << 11 | (1 << 7)
	csrw	mstatus, t0
	# Allow for MSIP (Software interrupt)
	li		t3, (1 << 3)
	csrw	mie, t3
	# Machine's exception program counter (MEPC) is set to the waiting loop.
	la		t1, kinit_hart
	csrw	mepc, t1
	# Machine's trap vector base address is set to `m_trap_vector`, for
	# "machine" trap vector.
	la		t2, m_trap_vector
	csrw	mtvec, t2

	la		ra, 4f
	# We use mret here so that the mstatus register is properly updated.
	mret

4:
	wfi
	j		4b


