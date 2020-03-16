# trap.S
# In the future our trap vector will go here.

.global asm_trap_vector
# This will be our trap vector when we start
# handling interrupts.
asm_trap_vector:
	csrr	a0, mtval
	wfi
	j asm_trap_vector

