use crate::syscall::write::stdout;


pub fn printk(msg: &str) {
    crate::write!(stdout, msg, msg.len());
}
