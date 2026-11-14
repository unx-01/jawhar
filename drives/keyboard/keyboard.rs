#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::asm;

const KEYBOARD_DATA: u16 = 0x60;
const KEYBOARD_STATUS: u16 = 0x64;

const NORMAL: [u8; 0x40] = [
    0x00,
    0x1B,
    b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',
    b'-', b'=',
    0x08,
    b'\t',
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p',
    b'[', b']',
    b'\n',
    0x00,
    b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l',
    b';', b'\'', b'`',
    0x00,
    b'\\',
    b'z', b'x', b'c', b'v', b'b', b'n', b'm',
    b',', b'.', b'/',
    0x00,
    0x00,
    0x00,
    b' ',
    0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
];

const SHIFTED: [u8; 0x40] = [
    0x00,
    0x1B,
    b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',
    b'_', b'+',
    0x08,
    b'\t',
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P',
    b'{', b'}',
    b'\n',
    0x00,
    b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L',
    b':', b'"', b'~',
    0x00,
    b'|',
    b'Z', b'X', b'C', b'V', b'B', b'N', b'M',
    b'<', b'>', b'?',
    0x00,
    0x00,
    0x00,
    b' ',
    0x00,
    0x00, 0x00, 0x00, 0x00, 0x00,
];

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static SHIFT: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static CAPS: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn status() -> u8 {
    let mut value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") KEYBOARD_STATUS, options(nomem, nostack));
    }
    value
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn read_data() -> u8 {
    let mut value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") KEYBOARD_DATA, options(nomem, nostack));
    }
    value
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn poll() -> Option<u8> {
    let status = status();
    if status & 0x01 == 0 {
        return None;
    }

    let scancode = read_data();
    if status & 0x20 != 0 {
        return None;
    }

    match scancode {
        0x2A | 0x36 => {
            SHIFT.store(true, Ordering::Relaxed);
            return None;
        }
        0xAA | 0xB6 => {
            SHIFT.store(false, Ordering::Relaxed);
            return None;
        }
        0x3A => {
            CAPS.fetch_xor(true, Ordering::Relaxed);
            return None;
        }
        0xE0 => return None,
        _ => {}
    }

    if scancode & 0x80 != 0 {
        return None;
    }

    let index = scancode as usize;
    if index >= NORMAL.len() {
        return None;
    }

    let mut byte = NORMAL[index];
    if byte == 0 {
        return None;
    }

    let shift = SHIFT.load(Ordering::Relaxed);
    let caps = CAPS.load(Ordering::Relaxed);

    if byte.is_ascii_alphabetic() {
        if shift ^ caps {
            byte = byte.to_ascii_uppercase();
        }
    } else if shift {
        byte = SHIFTED[index];
    }

    if byte == 0 { None } else { Some(byte) }
}

#[cfg(target_arch = "aarch64")]
pub fn poll() -> Option<u8> {
    None
}
