// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

#[no_mangle]
extern "C"
fn s_trap(epc: usize, tval: usize, cause: usize) -> usize {
	epc
}

#[no_mangle]
extern "C"
fn m_trap(epc: usize, tval: usize, cause: usize, hart: usize) -> usize {
	epc
}