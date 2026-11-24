#![allow(dead_code)]

use crate::pmm;
use crate::vmm;
use core::ptr;

extern "C" {
    static ap_trampoline_start: u8;
    static ap_trampoline_end: u8;
    static ap_tr_gdt: u8;
    static ap_tr_gdt_desc: u8;
    static ap_tr_cr3: u32;
    static ap_tr_stack: u32;
    static ap_tr_entry32: u32;
    fn ap_start32();
}

#[repr(C, packed)]
struct GdtDescPtr {
    limit: u16,
    base: u32,
}

const TRAMPOLINE_PHYS: usize = 0x8000;
const AP_STACK_PAGES: usize = 4;

pub unsafe fn start_ap(apic_id: u32) -> bool {
    let src_start = ptr::addr_of!(ap_trampoline_start) as usize;
    let src_end = ptr::addr_of!(ap_trampoline_end) as usize;
    let size = src_end.saturating_sub(src_start);
    if size == 0 || size > 4096 {
        return false;
    }

    ptr::copy_nonoverlapping(src_start as *const u8, TRAMPOLINE_PHYS as *mut u8, size);

    let off = |sym: usize| -> usize { sym - src_start };
    let gdt_off = off(ptr::addr_of!(ap_tr_gdt) as usize);
    let desc_off = off(ptr::addr_of!(ap_tr_gdt_desc) as usize);
    let cr3_off = off(ptr::addr_of!(ap_tr_cr3) as usize);
    let stack_off = off(ptr::addr_of!(ap_tr_stack) as usize);
    let entry_off = off(ptr::addr_of!(ap_tr_entry32) as usize);

    let desc_ptr = (TRAMPOLINE_PHYS + desc_off) as *mut GdtDescPtr;
    (*desc_ptr).base = (TRAMPOLINE_PHYS + gdt_off) as u32;

    let cr3 = vmm::AddressSpace::kernel().root() as u32;
    ptr::write((TRAMPOLINE_PHYS + cr3_off) as *mut u32, cr3);

    let Some(stack_phys) = pmm::alloc_pages(AP_STACK_PAGES) else {
        return false;
    };
    let stack_top = stack_phys + AP_STACK_PAGES * pmm::PAGE_SIZE;
    ptr::write((TRAMPOLINE_PHYS + stack_off) as *mut u32, stack_top as u32);
    ptr::write(
        (TRAMPOLINE_PHYS + entry_off) as *mut u32,
        ap_start32 as *const () as usize as u32,
    );

    let vector = (TRAMPOLINE_PHYS >> 12) as u8;
    super::lapic::init(super::lapic::LAPIC_BASE_DEFAULT);
    super::lapic::send_init(apic_id);
    delay(10_000);
    super::lapic::send_startup(apic_id, vector);
    delay(1_000);
    super::lapic::send_startup(apic_id, vector);
    delay(1_000);

    super::smp::ap_started()
}

fn delay(mut count: usize) {
    while count > 0 {
        core::hint::spin_loop();
        count -= 1;
    }
}
