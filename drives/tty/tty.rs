/*
 * tty
 */
use crate::{io, keyboard};

pub fn tty() {
    io::println("tty");
    let mut buf = [0u8; 128];
    loop {
        let _ = readline(&mut buf);
    }
}

fn wait_byte() -> u8 {
    loop {
        if let Some(b) = io::pollc() {
            return b;
        }
        if let Some(b) = keyboard::poll() {
            return b;
        }
        core::hint::spin_loop();
    }
}

fn readline(buf: &mut [u8]) -> usize {
    let mut len = 0;

    loop {
        let b = wait_byte();
        match b {
            b'\r' | b'\n' => {
                io::print("\n");
                return len;
            }
            0x08 | 0x7F => {
                if len > 0 {
                    len -= 1;
                    io::print("\x08 \x08");
                }
            }
            _ => {
                if len < buf.len() {
                    buf[len] = b;
                    len += 1;
                    io::put_byte(b);
                }
            }
        }
    }
}
