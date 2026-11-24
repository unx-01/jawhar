use crate::{errno, io};

pub const STDOUT_FILENO: i32 = 1;
pub const STDERR_FILENO: i32 = 2;

#[allow(non_upper_case_globals)]
pub const stdout: i32 = STDOUT_FILENO;

pub fn do_write(fd: i32, buf: *const u8, count: usize) -> isize {
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        errno::set_errno(errno::EBADF);
        return -1;
    }

    if count == 0 {
        return 0;
    }

    if buf.is_null() {
        errno::set_errno(errno::EFAULT);
        return -1;
    }

    let mut i = 0usize;
    while i < count {
        let byte = unsafe { *buf.add(i) };
        io::put_byte(byte);
        i += 1;
    }

    count as isize
}

#[macro_export]
macro_rules! write {
    ($fd:expr, $buf:expr, $count:expr $(,)?) => {{
        let __buf = $buf;
        $crate::syscall::write::do_write(
            $fd as i32,
            __buf.as_ptr() as *const u8,
            $count as usize,
        )
    }};
}
