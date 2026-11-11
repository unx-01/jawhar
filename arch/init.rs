#![no_std]
#![no_main]

#[path = "mod.rs"]
mod arch;

#[path = "../lib/io.rs"]
mod io;

#[path = "../drives/vga/tty.rs"]
mod tty;

#[path = "../mm/mem.rs"]
mod mem;

#[no_mangle]
pub extern "C" fn __init(a0: usize, a1: usize) -> ! {
    let _ = (a0, a1);   // معاملات Multiboot/DTB — تُستخدم لاحقًا
    let _ = mem::memcpy;

    arch::init();
    io::println("jawhar");
    tty::tty();

    arch::halt_forever();
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    arch::halt_forever();
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}
