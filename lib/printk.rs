use crate::io;

pub fn printk(level: u8, msg: &str) {
    if level <= 9 {
        io::print(msg);
    }
}
