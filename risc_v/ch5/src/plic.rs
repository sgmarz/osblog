// plic.rs
// Platform Level Interrupt Controller (PLIC)
// Stephen Marz
// 1 Nov 2019

const PLIC_PRIORITY: usize = 0x0c00_0000;
const PLIC_PENDING: usize = 0x0c00_1000;
const PLIC_INT_ENABLE: usize = 0x0c00_2000;
const PLIC_THRESHOLD: usize = 0x0c20_0000;
const PLIC_CLAIM: usize = 0x0c20_0004;

// Each register is 4-bytes (u32)
// The PLIC is an external interrupt controller. The one
// used by QEMU virt is the same as the SiFive PLIC.
// https://sifive.cdn.prismic.io/sifive%2F834354f0-08e6-423c-bf1f-0cb58ef14061_fu540-c000-v1.0.pdf

// Chapter 10 explains the priority, pending, interrupt enable, threshold and claims

// The virt machine has the following external interrupts (from Qemu source):
// Interrupt 0 is a "null" interrupt and is hardwired to 0.
// VIRTIO = [1..8]
// UART0 = 10
// PCIE = [32..35]


/// Get the next available interrupt. This is the "claim" process.
/// The plic will automatically sort by priority and hand us the
/// ID of the interrupt. For example, if the UART is interrupting
/// and it's next, we will get the value 10.
pub fn next() -> Option<u32> {
    let claim_reg = PLIC_CLAIM as *const u32;
    let claim_no;
    unsafe {
        claim_no = claim_reg.read_volatile();
    }
    if claim_no == 0 {
        None
    }
    else {
        Some(claim_no)
    }
}

/// Complete a pending interrupt by id. The id should come
/// from the next() function above.
pub fn complete(id: u32) {
    let complete_reg = PLIC_CLAIM as *mut u32;
    unsafe {
        complete_reg.write_volatile(id);
    }
}

/// Set the global threshold. The threshold can be a value [0..7].
/// The PLIC will mask any interrupts at or below the given threshold.
/// This means that a threshold of 7 will mask ALL interrupts and
/// a threshold of 0 will allow ALL interrupts.
pub fn set_threshold(tsh: u8) {
    let actual_tsh = tsh & 7;
    let tsh_reg = PLIC_THRESHOLD as *mut u32;
    unsafe {
        tsh_reg.write_volatile(actual_tsh as u32);
    }
}

/// See if a given interrupt id is pending.
pub fn is_pending(id: u32) -> bool {
    let pend = PLIC_PENDING as *const u32;
    let actual_id = 1 << id;
    let pend_ids;
    unsafe {
        pend_ids = pend.read_volatile();
    }
    actual_id & pend_ids != 0
}

/// Enable a given interrupt id
pub fn enable(id: u32) {
    let enables = PLIC_INT_ENABLE as *mut u32;
    let actual_id = 1 << id;
    unsafe {
        enables.write_volatile(enables.read_volatile() | actual_id);
    }
}

/// Set a given interrupt priority to the given priority.
/// The priority must be [0..7]
pub fn set_priority(id: u32, prio: u8) {
    let actual_prio = prio as u32 & 7;
    let prio_reg = PLIC_PRIORITY as *mut u32;
    unsafe {
        // The offset for the interrupt id is:
        // PLIC_PRIORITY + 4 * id
        // Since we're using pointer arithmetic on a u32 type,
        // it will automatically multiply the id by 4.
        prio_reg.add(id as usize).write_volatile(actual_prio);
    }
}

