#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering};

extern "C" {
    fn ap_entry();
}

static BSP_MPIDR: AtomicUsize = AtomicUsize::new(0);

pub fn init_bsp() {
    let mpidr = read_mpidr();
    BSP_MPIDR.store(mpidr, Ordering::Relaxed);
    super::super::smp::set_cpu_info(0, mpidr, mpidr);
}

pub fn cpu_id() -> usize {
    let mpidr = read_mpidr();
    if mpidr == BSP_MPIDR.load(Ordering::Relaxed) {
        0
    } else {
        // Logical IDs assigned on AP bring-up path.
        (mpidr & 0xFF) as usize
    }
}

#[inline]
pub fn read_mpidr() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) value, options(nostack, nomem));
    }
    value & 0x00FF_FFFF
}

pub unsafe fn start_secondary(cpu: usize, stack_top: usize) -> i64 {
    // PSCI CPU_ON (SMC64): fid=0xC4000003, target_cpu=mpidr affinity, entry, context
    let entry = ap_entry as *const () as usize as u64;
    psci_cpu_on(cpu as u64, entry, stack_top as u64)
}

pub unsafe fn psci_cpu_on(target_cpu: u64, entry: u64, context: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "mov x0, {fid}",
        "mov x1, {cpu}",
        "mov x2, {entry}",
        "mov x3, {ctx}",
        "hvc #0",
        fid = const 0xC400_0003u64,
        cpu = in(reg) target_cpu,
        entry = in(reg) entry,
        ctx = in(reg) context,
        out("x0") ret,
        out("x1") _,
        out("x2") _,
        out("x3") _,
        options(nostack),
    );
    ret
}

#[no_mangle]
pub unsafe extern "C" fn ap_entry_rust() -> ! {
    let mpidr = read_mpidr();
    let id = super::super::smp::alloc_logical_cpu().unwrap_or((mpidr & 0xFF) as usize);
    super::super::smp::set_cpu_info(id, mpidr, mpidr);
    super::super::smp::mark_online(id);

    loop {
        core::arch::asm!("wfe", options(nomem, nostack, preserves_flags));
    }
}
