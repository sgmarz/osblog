# trap.S
# In the future our trap vector will go here.

.global asm_trap_vector
asm_trap_vector:
	mret
