use crate::console;


pub fn tty() {
    loop {
        let line = console::readline("");
        crate::write!(1, line, line.len());
        crate::write!(1, "\n", 1);
    }
}
