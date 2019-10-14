# trap.S
# Trap handler and global context
# Steve Operating System
# Stephen Marz
# 24 February 2019
.option norvc
.altmacro
.set NUM_GP_REGS, 32  # Number of registers per context
.set NUM_FP_REGS, 32
.set REG_SIZE, 8   # Register size (in bytes)
.set MAX_CPUS, 8   # Maximum number of CPUs

.section .bss
.global _GLOBAL_CTX
.lcomm _GLOBAL_CTX, (NUM_FP_REGS + NUM_GP_REGS) * REG_SIZE * MAX_CPUS

# Use macros for saving and restoring multiple registers
.macro save_gp i, basereg=t6
	sd	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro load_gp i, basereg=t6
	ld	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro save_fp i, basereg=t6
	fsd	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm
.macro load_fp i, basereg=t6
	fld	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm


.section .text
.global m_trap_vector
# This must be aligned by 4 since the last two bits
# of the mtvec register do not contribute to the address
# of this vector.
.align 4
m_trap_vector:
	# We have to save t6 (x31), since it will be our
	# base register for saving
	csrw	mscratch, t6

	csrr	t6, satp
	slli	t6, t6, 1
	srli	t6, t6, 1
	csrw	satp, t6
    # Start saving the registers. We can't do much until the registers
    # have been saved.
	la		t6, _GLOBAL_CTX

	# Set i = 1. i = 0 is the zero register and is a waste of time.
	.set i, 1
	.rept 30
		save_gp	%i
		.set	i, i+1
	.endr
	# We saved 31 registers. All but t6
	# t5 is saved, so we can overwrite it.
	mv		t5, t6
	csrr	t6, mscratch
	save_gp	31, t5

	# We need to check the status of the floating
	# point system. (FS bits are described in 3.1.6.5
	# and table 3.3 in the RISC-V Privilege spec).
	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	
	# Skip saving the FP registers if the FP is turned off.
	beqz 	t0, 3f
	la		t6, _GLOBAL_CTX
	.set i, 0
	.rept 32
		save_fp	%i
		.set	i, i+1
	.endr

3:
	# Go to Rust
	# usize trap_handler(mepc, mcause)
	# trap_handler returns the new mepc
	# via a0
	la		sp, _stack_end
	li		t0, 0x10000
	csrr	t1, mhartid
	mul		t0, t0, t1
	sub		sp, sp, t0
	csrr	a0, mepc
	csrr	a1, mtval
	csrr	a2, mcause
	csrr	a3, mhartid
	csrr	a4, mstatus
	la		t1, m_trap
	csrw	mepc, t1
	la		ra, 3f
	li		t2, 0b11 << 11
	csrw	mstatus, t2
	mret
3:
	csrw	mepc, a0

	# Move to the context for this CPU
	la		t6, _GLOBAL_CTX
	# Go to the context for this hart
	csrr	t5, mhartid
	slli	t5, t5, 6
	add		t6, t6, t5

	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3

	# Skip loading the FP registers if the FP is turned off.
	beqz	t0, 3f
	.set i, 0
	.rept 32
		load_fp	%i
		.set	i, i+1
	.endr

3:
	# Restore the registers
	.set i, 1
	.rept 31
		load_gp %i
		.set i, i+1
	.endr

	# The MMU should be going at this point via
	# Rust.

	li		t2, (1 << 8) | (1 << 5)
	csrw	mstatus, t2

	# x31 gets restored at the last iteration
	sret

.global make_syscall
make_syscall:
	ecall
	ret
