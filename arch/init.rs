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

#[path = "../mm/pmm.rs"]
mod pmm;

#[path = "../mm/vmm.rs"]
mod vmm;

#[path = "../process/init.rs"]
mod process;

#[path = "../kernel/mod.rs"]
mod syscall;

#[path = "../lib/spinlock.rs"]
mod spinlock;

#[path = "../lib/time.rs"]
mod time;

#[path = "../lib/errno.rs"]
mod errno;

static mut BOOT_LOG: [u8; 256] = [0; 256];
static mut BOOT_LOG_LEN: usize = 0;

unsafe fn append(buf: *mut u8, len: *mut usize, capacity: usize, src: *const u8, count: usize) {
    let available = capacity - *len;
    let count = count.min(available);
    core::ptr::copy_nonoverlapping(src, buf.add(*len), count);
    *len += count;
}

unsafe fn append_str(buf: *mut u8, len: *mut usize, capacity: usize, s: &[u8]) {
    append(buf, len, capacity, s.as_ptr(), s.len());
}

unsafe fn append_num(buf: *mut u8, len: *mut usize, capacity: usize, value: usize) {
    let mut tmp = [0u8; 24];
    let mut p = tmp.as_mut_ptr().add(tmp.len());
    let mut count = 0usize;
    let mut n = value;

    loop {
        p = p.sub(1);
        *p = b'0' + (n % 10) as u8;
        count += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }

    append(buf, len, capacity, p, count);
}

unsafe fn append_padded(buf: *mut u8, len: *mut usize, capacity: usize, value: usize, width: usize) {
    let mut tmp = [0u8; 24];
    let mut p = tmp.as_mut_ptr().add(tmp.len());
    let mut count = 0usize;
    let mut n = value;

    loop {
        p = p.sub(1);
        *p = b'0' + (n % 10) as u8;
        count += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }

    for _ in count..width {
        append_str(buf, len, capacity, b"0");
    }
    append(buf, len, capacity, p, count);
}

fn log_service(name: &[u8], ms: usize) {
    unsafe {
        let at = arch::time::micros() as usize;
        let buf = core::ptr::addr_of_mut!(BOOT_LOG) as *mut u8;
        let len = core::ptr::addr_of_mut!(BOOT_LOG_LEN);
        let capacity = 256;
        append_str(buf, len, capacity, b"[      ");
        append_num(buf, len, capacity, (at / 1_000_000) as usize);
        append_str(buf, len, capacity, b".");
        append_padded(buf, len, capacity, (at % 1_000_000) as usize, 6);
        append_str(buf, len, capacity, b"] ");
        append_str(buf, len, capacity, name);
        append_str(buf, len, capacity, b"\n");
    }
}

fn boot_service<F: FnOnce()>(name: &[u8], service: F) {
    let start = arch::time::millis();
    service();
    let elapsed = arch::time::millis().saturating_sub(start);
    log_service(name, elapsed as usize);
}

fn flush_boot_log() {
    unsafe {
        let len = core::ptr::addr_of!(BOOT_LOG_LEN).read();
        if len == 0 {
            return;
        }
        let bytes = core::slice::from_raw_parts(core::ptr::addr_of!(BOOT_LOG) as *const u8, len);
        let msg = core::str::from_utf8_unchecked(bytes);
        printk::printk(msg);
    }
}

#[no_mangle]
pub extern "C" fn __init(a0: usize, a1: usize) -> ! {
    let _ = mem::memcpy;

    let arch_start = arch::time::cycles();
    arch::init();
    let arch_ms = arch::time::cycles_to_ms(arch::time::cycles().saturating_sub(arch_start));
    log_service(b"arch", arch_ms as usize);

    boot_service(b"pmm", || pmm::pmm_init(a0, a1));
    boot_service(b"vmm", vmm::vmm_init);
    boot_service(b"process", process::init);
    boot_service(b"console", console::console);
    flush_boot_log();

    tty::tty();

    arch::halt_forever();
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    arch::halt_forever();
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

