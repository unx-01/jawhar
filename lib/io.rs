use core::fmt::{self, Write};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const SERIAL_PORT: u16 = 0x3F8;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn putc(b: u8) {
    core::arch::asm!("out dx, al", in("dx") SERIAL_PORT, in("al") b, options(nomem, nostack));
}

#[cfg(target_arch = "aarch64")]
unsafe fn putc(b: u8) {
    core::ptr::write_volatile(0x0900_0000 as *mut u8, b);
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn getc() -> u8 {
    let mut b: u8;
    loop {
        core::arch::asm!("in al, dx", out("al") b, in("dx") SERIAL_PORT, options(nomem, nostack));
        if b != 0 {
            return b;
        }
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn getc() -> u8 {
    loop {
        let b = core::ptr::read_volatile(0x0900_0000 as *const u8);
        if b != 0 {
            return b;
        }
    }
}

pub fn gets(buf: &mut [u8]) -> usize {
    let mut len = 0;
    loop {
        let b = unsafe { getc() };
        match b {
            b'\r' | b'\n' => {
                print("\n");
                return len;
            }
            0x08 | 0x7F => {
                if len > 0 {
                    len = len.wrapping_sub(1);
                    print("\x08 \x08");
                }
            }
            _ => {
                if len < buf.len() {
                    unsafe { *buf.get_unchecked_mut(len) = b; }
                    len = len.wrapping_add(1);
                    let mut uart = Uart;
                    let _ = uart.write_str(unsafe {
                        core::str::from_utf8_unchecked(core::slice::from_ref(&b))
                    });
                }
            }
        }
    }
}
