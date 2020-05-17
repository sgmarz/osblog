# trap.S
# Trap handler and global context
# Steve Operating System
# Stephen Marz
# 24 February 2019
.option norvc
.altmacro
.set NUM_GP_REGS, 32  # Number of registers per context
.set REG_SIZE, 8   # Register size (in bytes)

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
	# We use t6 as the temporary register because it is the very
	# bottom register (x31)
	.set 	i, 0
	.rept	31
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

	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li		t3, 3
	bne		t0, t3, 1f
	# Save floating point registers
	.set 	i, 0
	.rept	32
		save_fp	%i, t5
		.set	i, i+1
	.endr
1:
	# Get ready to go into Rust (trap.rs)
	# We don't want to write into the user's stack or whomever
	# messed with us here.
	# csrw	mie, zero

	csrr	a0, mepc
	sd		a0, 520(t5)
	csrr	a1, mtval
	csrr	a2, mcause
	csrr	a3, mhartid
	csrr	a4, mstatus
	csrr	a5, mscratch
	la		t0, KERNEL_STACK_END
	ld		sp, 0(t0)
	call	m_trap

	# When we get here, we've returned from m_trap, restore registers
	# and return.
	# m_trap will return the return address via a0.

	csrw	mepc, a0
	# Now load the trap frame back into t6
	csrr	t6, mscratch

	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li		t3, 3
	bne		t0, t3, 1f
	.set	i, 0
	.rept	32
		load_fp %i
		.set i, i+1
	.endr
1:
	# Restore all GP registers
	.set	i, 1
	.rept	31
		load_gp %i
		.set	i, i+1
	.endr

	# Since we ran this loop 31 times starting with i = 1,
	# the last one loaded t6 back to its original value.
	mret

.global switch_to_user
switch_to_user:
    # a0 - Frame address
	# a1 - Program counter
	# a2 - SATP Register
    csrw    mscratch, a0

	# Load program counter
	ld		a1, 520(a0)
	# Load satp
	ld		a2, 512(a0)
	# Load processor mode
	ld		a3, 552(a0)
	# Pid
	# ld		a4, 544(a0)

	# 1 << 7 is MPIE
	# Since user mode is 00, we don't need to set anything
	# in MPP (bits 12:11)
	li		t0, 1 << 7 | 1 << 5 | 1 << 13
	# Combine enable bits with mode bits.
	slli	a3, a3, 11
	or		t0, t0, a3
	csrw	mstatus, t0
	csrw	mepc, a1
	csrw	satp, a2
	li		t1, 0xaaa
	csrw	mie, t1
	la		t2, m_trap_vector
	csrw	mtvec, t2
	# This fence forces the MMU to flush the TLB. However, since
	# we're using the PID as the address space identifier, we might
	# only need this when we create a process. Right now, this ensures
	# correctness, however it isn't the most efficient.
	# sfence.vma
	# A0 is the context frame, so we need to reload it back
	# and mret so we can start running the program.
	mv	t6, a0
	.set	i, 0
	.rept	32
		load_fp %i
		.set i, i+1
	.endr
1:
	.set	i, 1
	.rept	31
		load_gp %i, t6
		.set	i, i+1
	.endr

    mret


.global make_syscall
make_syscall:
	# We're setting this up to work with libgloss
	# They want a7 to be the system call number and all parameters
	# in a0 - a5
	mv	a7, a0
	mv	a0, a1
	mv	a1, a2
	mv	a2, a3
	mv	a3, a4
	mv	a4, a5
	mv	a5, a6
	ecall
	ret
