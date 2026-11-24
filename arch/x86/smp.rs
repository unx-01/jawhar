#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static BSP_APIC_ID: AtomicUsize = AtomicUsize::new(0);
static AP_STARTED: AtomicBool = AtomicBool::new(false);

pub fn init_bsp() {
    let apic_id = read_apic_id();
    BSP_APIC_ID.store(apic_id, Ordering::Relaxed);
    super::super::smp::set_cpu_info(0, apic_id, 0);
}

pub fn cpu_id() -> usize {
    let apic = read_apic_id();
    if apic == BSP_APIC_ID.load(Ordering::Relaxed) { 0 } else { 0 }
}

pub fn read_apic_id() -> usize {
    let ebx: u32;
    unsafe {
        core::arch::asm!(
            "push ebx",
            "mov eax, 1",
            "cpuid",
            "mov {out:e}, ebx",
            "pop ebx",
            out = out(reg) ebx,
            out("eax") _,
            out("ecx") _,
            out("edx") _,
            options(nostack),
        );
    }
    ((ebx >> 24) & 0xFF) as usize
}

#[no_mangle]
pub unsafe extern "C" fn ap_entry_rust() -> ! {
    let id = super::super::smp::alloc_logical_cpu().unwrap_or(0);
    let apic = read_apic_id();
    super::super::smp::set_cpu_info(id, apic, 0);
    super::super::smp::mark_online(id);
    AP_STARTED.store(true, Ordering::Release);
    loop {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

pub fn ap_started() -> bool {
    AP_STARTED.load(Ordering::Acquire)
}
