// mem.S
// Importation of linker symbols

.section .rodata
.global HEAP_START
HEAP_START: .dword _heap_start

.global HEAP_SIZE
HEAP_SIZE: .dword _heap_size
