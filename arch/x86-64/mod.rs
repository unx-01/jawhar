pub mod gdt;
pub mod sys;

pub fn init() {
    gdt::init();
}
