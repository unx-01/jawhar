#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};

pub const LAPIC_BASE_DEFAULT: usize = 0xFEE0_0000;

const REG_ID: usize = 0x020;
const REG_EOI: usize = 0x0B0;
const REG_SVR: usize = 0x0F0;
const REG_ICR_LOW: usize = 0x300;
const REG_ICR_HIGH: usize = 0x310;

const SVR_ENABLE: u32 = 1 << 8;
const ICR_DELIVERY_INIT: u32 = 0b101 << 8;
const ICR_DELIVERY_STARTUP: u32 = 0b110 << 8;
const ICR_LEVEL_ASSERT: u32 = 1 << 14;
const ICR_LEVEL_DEASSERT: u32 = 0 << 14;
const ICR_TRIGGER_LEVEL: u32 = 1 << 15;
const ICR_TRIGGER_EDGE: u32 = 0 << 15;
const ICR_DEST_PHYSICAL: u32 = 0 << 11;
const ICR_PENDING: u32 = 1 << 12;

static mut LAPIC_BASE: usize = LAPIC_BASE_DEFAULT;

#[inline]
unsafe fn reg(offset: usize) -> *mut u32 {
    (LAPIC_BASE + offset) as *mut u32
}

pub unsafe fn init(base: usize) {
    LAPIC_BASE = if base == 0 { LAPIC_BASE_DEFAULT } else { base };
    write_volatile(reg(REG_SVR), SVR_ENABLE | 0xFF);
}

pub unsafe fn id() -> u32 {
    read_volatile(reg(REG_ID)) >> 24
}

pub unsafe fn eoi() {
    write_volatile(reg(REG_EOI), 0);
}

unsafe fn wait_icr() {
    while read_volatile(reg(REG_ICR_LOW)) & ICR_PENDING != 0 {
        core::hint::spin_loop();
    }
}

pub unsafe fn send_init(apic_id: u32) {
    wait_icr();
    write_volatile(reg(REG_ICR_HIGH), apic_id << 24);
    write_volatile(
        reg(REG_ICR_LOW),
        ICR_DEST_PHYSICAL | ICR_DELIVERY_INIT | ICR_LEVEL_ASSERT | ICR_TRIGGER_LEVEL,
    );
    wait_icr();
    write_volatile(reg(REG_ICR_HIGH), apic_id << 24);
    write_volatile(
        reg(REG_ICR_LOW),
        ICR_DEST_PHYSICAL | ICR_DELIVERY_INIT | ICR_LEVEL_DEASSERT | ICR_TRIGGER_LEVEL,
    );
    wait_icr();
}

pub unsafe fn send_startup(apic_id: u32, vector: u8) {
    wait_icr();
    write_volatile(reg(REG_ICR_HIGH), apic_id << 24);
    write_volatile(
        reg(REG_ICR_LOW),
        ICR_DEST_PHYSICAL | ICR_DELIVERY_STARTUP | ICR_TRIGGER_EDGE | (vector as u32),
    );
    wait_icr();
}
