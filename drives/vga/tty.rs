/*
 * tty
 */
use crate::io;

pub fn tty()
{
    io::println("tty");
    let mut buf = [0u8; 128];
    loop {
        let _ = io::gets(&mut buf);
    }
}
