#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::asm;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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

#[derive(Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub sup: bool,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static SHIFT: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static CTRL: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static ALT: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static SUPER: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static CAPS: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static EXTENDED: AtomicBool = AtomicBool::new(false);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const QUEUE_SIZE: usize = 16;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut QUEUE: [u8; QUEUE_SIZE] = [0; QUEUE_SIZE];

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static QUEUE_HEAD: AtomicUsize = AtomicUsize::new(0);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static QUEUE_TAIL: AtomicUsize = AtomicUsize::new(0);

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn queue_byte(byte: u8) {
    let tail = QUEUE_TAIL.load(Ordering::Relaxed);
    let next = (tail + 1) % QUEUE_SIZE;
    if next == QUEUE_HEAD.load(Ordering::Relaxed) {
        return;
    }
    unsafe {
        let queue = core::ptr::addr_of_mut!(QUEUE) as *mut u8;
        core::ptr::write(queue.add(tail), byte);
    }
    QUEUE_TAIL.store(next, Ordering::Relaxed);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn queue_bytes(bytes: &[u8]) {
    for byte in bytes {
        queue_byte(*byte);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn dequeue_byte() -> Option<u8> {
    let head = QUEUE_HEAD.load(Ordering::Relaxed);
    if head == QUEUE_TAIL.load(Ordering::Relaxed) {
        return None;
    }
    let byte = unsafe {
        let queue = core::ptr::addr_of!(QUEUE) as *const u8;
        core::ptr::read(queue.add(head))
    };
    QUEUE_HEAD.store((head + 1) % QUEUE_SIZE, Ordering::Relaxed);
    Some(byte)
}

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

pub fn modifiers() -> Modifiers {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        Modifiers {
            shift: SHIFT.load(Ordering::Relaxed),
            ctrl: CTRL.load(Ordering::Relaxed),
            alt: ALT.load(Ordering::Relaxed),
            sup: SUPER.load(Ordering::Relaxed),
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        Modifiers::default()
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn control(byte: u8) -> Option<u8> {
    match byte {
        b'a'..=b'z' => Some(byte - b'a' + 1),
        b'A'..=b'Z' => Some(byte - b'A' + 1),
        b'@' | b' ' => Some(0x00),
        b'[' => Some(0x1B),
        b'\\' => Some(0x1C),
        b']' => Some(0x1D),
        b'^' => Some(0x1E),
        b'_' => Some(0x1F),
        b'?' => Some(0x7F),
        _ => None,
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn poll() -> Option<u8> {
    if let Some(byte) = dequeue_byte() {
        return Some(byte);
    }

    let status = status();
    if status & 0x01 == 0 {
        return None;
    }

    let scancode = read_data();
    if status & 0x20 != 0 {
        return None;
    }

    if scancode == 0xE0 {
        EXTENDED.store(true, Ordering::Relaxed);
        return None;
    }

    let extended = EXTENDED.swap(false, Ordering::Relaxed);
    let released = scancode & 0x80 != 0;
    let code = scancode & 0x7F;

    if extended {
        match code {
            0x1D => CTRL.store(!released, Ordering::Relaxed),
            0x38 => ALT.store(!released, Ordering::Relaxed),
            0x5B | 0x5C => SUPER.store(!released, Ordering::Relaxed),
            0x48 | 0x50 | 0x4D | 0x4B | 0x47 | 0x4F | 0x52 | 0x53 | 0x49 | 0x51 => {
                if !released {
                    match code {
                        0x48 => queue_bytes(b"[A"),
                        0x50 => queue_bytes(b"[B"),
                        0x4D => queue_bytes(b"[C"),
                        0x4B => queue_bytes(b"[D"),
                        0x47 => queue_bytes(b"[H"),
                        0x4F => queue_bytes(b"[F"),
                        0x52 => queue_bytes(b"[2~"),
                        0x53 => queue_bytes(b"[3~"),
                        0x49 => queue_bytes(b"[5~"),
                        0x51 => queue_bytes(b"[6~"),
                        _ => {}
                    }
                    return Some(0x1B);
                }
            }
            _ => {}
        }
        return None;
    }

    match code {
        0x2A | 0x36 => {
            SHIFT.store(!released, Ordering::Relaxed);
            return None;
        }
        0x1D => {
            CTRL.store(!released, Ordering::Relaxed);
            return None;
        }
        0x38 => {
            ALT.store(!released, Ordering::Relaxed);
            return None;
        }
        0x5B | 0x5C => {
            SUPER.store(!released, Ordering::Relaxed);
            return None;
        }
        0x3A => {
            if !released {
                CAPS.fetch_xor(true, Ordering::Relaxed);
            }
            return None;
        }
        _ => {}
    }

    if released || code as usize >= NORMAL.len() {
        return None;
    }

    let mut byte = unsafe { *NORMAL.get_unchecked(code as usize) };
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
        byte = unsafe { *SHIFTED.get_unchecked(code as usize) };
    }

    if byte == 0 {
        return None;
    }

    let ctrl = CTRL.load(Ordering::Relaxed);
    let alt = ALT.load(Ordering::Relaxed);

    let output = if ctrl {
        control(byte).unwrap_or(byte)
    } else {
        byte
    };

    if alt {
        queue_byte(output);
        Some(0x1B)
    } else {
        Some(output)
    }
}

#[cfg(target_arch = "aarch64")]
pub fn poll() -> Option<u8> {
    None
}
