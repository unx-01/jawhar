pub mod gdt;
pub mod lapic;
pub mod smp;
pub mod ap_startup;

pub fn init() {
    gdt::init();
}
