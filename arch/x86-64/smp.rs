#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static BSP_APIC_ID: AtomicUsize = AtomicUsize::new(0);
static AP_STARTED: AtomicBool = AtomicBool::new(false);
static AP_STACK_TOP: AtomicUsize = AtomicUsize::new(0);
static AP_CR3: AtomicUsize = AtomicUsize::new(0);
static AP_ENTRY: AtomicUsize = AtomicUsize::new(0);

pub fn init_bsp() {
    let apic_id = read_apic_id();
    BSP_APIC_ID.store(apic_id, Ordering::Relaxed);
    super::super::smp::set_cpu_info(0, apic_id, 0);
    // SMP bring-up requires LAPIC/IOAPIC and ACPI MADT parsing.
    // Framework is initialized here; AP startup is wired for trampoline-based bring-up.
}

pub fn cpu_id() -> usize {
    // Until per-CPU GS/base is set up, BSP path is identified by LAPIC ID.
    let apic = read_apic_id();
    if apic == BSP_APIC_ID.load(Ordering::Relaxed) {
        0
    } else {
        // APs receive logical IDs during ap_entry.
        0
    }
}

pub fn read_apic_id() -> usize {
    // CPUID leaf 1: initial APIC ID in EBX[31:24]
    let ebx: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "mov eax, 1",
            "cpuid",
            "mov {out:e}, ebx",
            "pop rbx",
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
    // Called from AP trampoline/start path after stack setup.
    let id = super::super::smp::alloc_logical_cpu().unwrap_or(0);
    let apic = read_apic_id();
    super::super::smp::set_cpu_info(id, apic, 0);
    super::super::smp::mark_online(id);
    AP_STARTED.store(true, Ordering::Release);

    loop {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

pub fn set_ap_boot_params(stack_top: usize, cr3: usize, entry: usize) {
    AP_STACK_TOP.store(stack_top, Ordering::Relaxed);
    AP_CR3.store(cr3, Ordering::Relaxed);
    AP_ENTRY.store(entry, Ordering::Relaxed);
}

pub fn ap_boot_params() -> (usize, usize, usize) {
    (
        AP_STACK_TOP.load(Ordering::Relaxed),
        AP_CR3.load(Ordering::Relaxed),
        AP_ENTRY.load(Ordering::Relaxed),
    )
}

pub fn ap_started() -> bool {
    AP_STARTED.load(Ordering::Acquire)
}
