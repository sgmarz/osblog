# trap.S
# Trap handler and global context
# Steve Operating System
# Stephen Marz
# 24 February 2019
.option norvc
.altmacro
.set NUM_GP_REGS, 34  # Number of registers per context
.set NUM_FP_REGS, 32
.set REG_SIZE, 8   # Register size (in bytes)

.section .bss
.global _GLOBAL_CTX
.lcomm _GLOBAL_CTX, (NUM_FP_REGS + NUM_GP_REGS) * REG_SIZE

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

	# Disable the MMU since we're switching.
	csrr	t6, satp
	slli	t6, t6, 4
	srli	t6, t6, 4
	csrw	satp, t6

	# Disable interrupts
	li		t6, 1 << 3
	csrrc	zero, mstatus, t6

    # Start saving the registers. We can't do much until the registers
    # have been saved.
	la		t6, _GLOBAL_CTX
	.set i, 0
	.rept 31
		save_gp	%i
		.set	i, i+1
	.endr
	# We saved 31 registers. All but t6
	# t5 is saved, so we can overwrite it.
	mv		t5, t6
	csrr	t6, mscratch
	save_gp	31, t5

	# csrr	t0, satp
	# sd		t0, 32*8(t5)

	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li	t1, 3
	
	# Skip saving the FP registers if the FP is turned off.
	bne	t0, t1, 3f
	la	t6, _GLOBAL_CTX
	.set i, 0
	.rept 32
		save_fp	%i
		.set	i, i+1
	.endr

3:
	# Go to C++
	# usize trap_handler(mepc, mcause)
	# trap_handler returns the new mepc
	# via a0
    la      sp, _stack_end
	csrr	a0, mepc
	csrr	a1, mtval
	csrr	a2, mcause
	csrr	a3, mhartid
	call	m_trap
	csrw	mepc, a0

	# Move to the context for this CPU
	la		t6, _GLOBAL_CTX
	csrr	t1, mstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li		t1, 3

	# Skip loading the FP registers if the FP is turned off.
	bne	t0, t1, 3f
	.set i, 0
	.rept 32
		load_fp	%i
		.set	i, i+1
	.endr

3:

	# csrr	t5, mhartid
	# slli	t5, t5, CTX_SHL
	# add	t6, t6, t5
	# Restore the registers
	.set i, 0
	.rept 32
		load_gp %i
		.set i, i+1
	.endr
	# Get the MMU going
	csrw	mscratch, t6
	la		t6, _GLOBAL_CTX
	ld		t6, 32*8(t6)
	csrw	satp, t6
	sfence.vma
	csrr	t6, mscratch
	# x31 gets restored at the last iteration
	mret


.global s_trap_vector
# This must be aligned by 4 since the last two bits
# of the mtvec register do not contribute to the address
# of this vector.
.align 4
s_trap_vector:
	# We have to save t6 (x31), since it will be our
	# base register for saving
	csrw	sscratch, t6

	# Disable the MMU since we're switching.
	csrr	t6, satp
	slli	t6, t6, 4
	srli	t6, t6, 4
	csrw	satp, t6

	# Disable interrupts
	li		t6, 1 << 3
	csrrc	zero, sstatus, t6

    # Start saving the registers. We can't do much until the registers
    # have been saved.
	la		t6, _GLOBAL_CTX
	.set i, 0
	.rept 31
		save_gp	%i
		.set	i, i+1
	.endr
	# We saved 31 registers. All but t6
	# t5 is saved, so we can overwrite it.
	mv		t5, t6
	csrr	t6, sscratch
	save_gp	31, t5

	# csrr	t0, satp
	# sd		t0, 32*8(t5)

	csrr	t1, sstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li		t1, 3
	
	# Skip saving the FP registers if the FP is turned off.
	bne	t0, t1, 3f
	la	t6, _GLOBAL_CTX
	.set i, 0
	.rept 32
		save_fp	%i
		.set	i, i+1
	.endr

3:
	# Go to C++
	# usize trap_handler(mepc, mcause)
	# trap_handler returns the new mepc
	# via a0
    la      sp, _stack_end
	csrr	a0, sepc
	csrr	a1, stval
	csrr	a2, scause
	call	s_trap
	csrw	sepc, a0

	# Move to the context for this CPU
	la		t6, _GLOBAL_CTX
	csrr	t1, sstatus
	srli	t0, t1, 13
	andi	t0, t0, 3
	li		t1, 3

	# Skip loading the FP registers if the FP is turned off.
	bne	t0, t1, 3f
	.set i, 0
	.rept 32
		load_fp	%i
		.set	i, i+1
	.endr

3:

	# csrr	t5, mhartid
	# slli	t5, t5, CTX_SHL
	# add	t6, t6, t5
	# Restore the registers
	.set i, 0
	.rept 32
		load_gp %i
		.set i, i+1
	.endr
	# Get the MMU going
	csrw	sscratch, t6
	la		t6, _GLOBAL_CTX
	ld		t6, 32*8(t6)
	csrw	satp, t6
	sfence.vma
	csrr	t6, mscratch
	# x31 gets restored at the last iteration
	sret



.global make_syscall
make_syscall:
	ecall
	ret
