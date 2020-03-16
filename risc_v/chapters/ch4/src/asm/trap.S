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
	# All registers are volatile here, we need to save them
	# before we do anything.
	csrrw	t6, mscratch, t6
	# csrrw will atomically swap t6 into mscratch and the old
	# value of mscratch into t6. This is nice because we just
	# switched values and didn't destroy anything -- all atomically!
	# in cpu.rs we have a structure of:
	#  32 gp regs		0
	#  32 fp regs		256
	#  SATP register	512
	#  Trap stack       520
	#  CPU HARTID		528
	# We use t6 as the temporary register because it is the very
	# bottom register (x31)
	.set 	i, 1
	.rept	30
		save_gp	%i
		.set	i, i+1
	.endr

	# Save the actual t6 register, which we swapped into
	# mscratch
	mv		t5, t6
	csrr	t6, mscratch
	save_gp 31, t5

	# Restore the kernel trap frame into mscratch
	csrw	mscratch, t5

	# Get ready to go into Rust (trap.rs)
	# We don't want to write into the user's stack or whomever
	# messed with us here.
	csrr	a0, mepc
	csrr	a1, mtval
	csrr	a2, mcause
	csrr	a3, mhartid
	csrr	a4, mstatus
	mv		a5, t5
	ld		sp, 520(a5)
	call	m_trap

	# When we get here, we've returned from m_trap, restore registers
	# and return.
	# m_trap will return the return address via a0.

	csrw	mepc, a0

	# Now load the trap frame back into t6
	csrr	t6, mscratch

	# Restore all GP registers
	.set	i, 1
	.rept	31
		load_gp %i
		.set	i, i+1
	.endr

	# Since we ran this loop 31 times starting with i = 1,
	# the last one loaded t6 back to its original value.

	mret


.global make_syscall
make_syscall:
	ecall
	ret
