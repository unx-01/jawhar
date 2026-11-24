use crate::{errno, process, time};

pub fn do_times(buf: *mut time::Tms) -> time::ClockT {
    if buf.is_null() {
        errno::set_errno(errno::EFAULT);
        return -1;
    }
    process::times(buf)
}

#[macro_export]
macro_rules! times {
    ($buf:expr $(,)?) => {{
        let __buf = $buf;
        $crate::syscall::times::do_times(__buf as *mut $crate::time::Tms)
    }};
}
