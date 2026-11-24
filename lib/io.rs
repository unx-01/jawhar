use core::fmt::{self, Write};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::asm;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const SERIAL_PORT: u16 = 0x3F8;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const SERIAL_LSR: u16 = SERIAL_PORT + 5;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn putc(b: u8) {
    loop {
        let mut lsr: u8;
        asm!("in al, dx", out("al") lsr, in("dx") SERIAL_LSR, options(nomem, nostack));
        if lsr & 0x20 != 0 {
            break;
        }
    }

    asm!("out dx, al", in("dx") SERIAL_PORT, in("al") b, options(nomem, nostack));
    crate::console::putc(b);
}

#[cfg(target_arch = "aarch64")]
const PL011_BASE: usize = 0x0900_0000;

#[cfg(target_arch = "aarch64")]
const PL011_FR: usize = PL011_BASE + 0x18;

#[cfg(target_arch = "aarch64")]
pub unsafe fn putc(b: u8) {
    loop {
        let fr = core::ptr::read_volatile(PL011_FR as *const u32);
        if fr & (1 << 5) == 0 {
            break;
        }
    }
    core::ptr::write_volatile(PL011_BASE as *mut u32, b as u32);
    crate::console::putc(b);
}

struct Uart;

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            unsafe { putc(b); }
        }
        Ok(())
    }
}

pub fn print(s: &str) {
    let mut uart = Uart;
    let _ = uart.write_str(s);
}

pub fn println(s: &str) {
    print(s);
    print("\r\n");
}

pub fn put_byte(b: u8) {
    unsafe { putc(b); }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn pollc() -> Option<u8> {
    unsafe {
        let mut lsr: u8;
        asm!("in al, dx", out("al") lsr, in("dx") SERIAL_LSR, options(nomem, nostack));
        if lsr & 0x01 != 0 {
            let mut b: u8;
            asm!("in al, dx", out("al") b, in("dx") SERIAL_PORT, options(nomem, nostack));
            Some(b)
        } else {
            None
        }
    }
}

#[cfg(target_arch = "aarch64")]
pub fn pollc() -> Option<u8> {
    unsafe {
        let fr = core::ptr::read_volatile(PL011_FR as *const u32);
        if fr & (1 << 4) == 0 {
            let dr = core::ptr::read_volatile(PL011_BASE as *const u32);
            Some(dr as u8)
        } else {
            None
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nostack, preserves_flags)
    );
    value
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn outb(port: u16, val: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nostack, preserves_flags)
    );
}
