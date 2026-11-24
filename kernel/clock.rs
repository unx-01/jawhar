use crate::{process, time};

pub fn do_clock() -> time::ClockT {
    process::clock()
}

#[macro_export]
macro_rules! clock {
    () => {
        $crate::syscall::clock::do_clock()
    };
}
