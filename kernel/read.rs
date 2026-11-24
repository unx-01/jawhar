#![allow(dead_code)]

pub fn do_read(_fd: u32, _buf: &mut [u8], _count: u64) -> isize {
    -1
}

#[macro_export]
macro_rules! read {
    ($fd:expr, $buf:expr, $count:expr $(,)?) => {{
        $crate::syscall::read::do_read($fd as u32, $buf, $count as u64)
    }};
}
