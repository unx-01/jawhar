#![no_std]
#![no_main]
#![allow(dead_code)]

#[path = "mod.rs"]
mod arch;

#[path = "../lib/io.rs"]
mod io;

#[path = "../drives/tty/tty.rs"]
mod tty;

#[path = "../drives/console/console.rs"]
mod console;

#[path = "../drives/keyboard/keyboard.rs"]
mod keyboard;

#[path = "../lib/printk.rs"]
mod printk;

#[path = "../mm/mem.rs"]
mod mem;

#[path = "../mm/vmm.rs"]
mod vmm;

#[path = "../process/init.rs"]
mod process;

#[no_mangle]
pub extern "C" fn __init(a0: usize, a1: usize) -> ! {
    let _ = (a0, a1);   // معاملات Multiboot/DTB — تُستخدم لاحقًا
    let _ = mem::memcpy;

    arch::init();
    console::console();
    tty::tty();

    arch::halt_forever();
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    arch::halt_forever();
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}
