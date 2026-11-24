pub const EXIT_SUCCESS: u16 = 0;
pub const EXIT_FAILURE: u16 = 1;

pub fn do_exit(status: u16) -> ! {
    crate::process::exit(status as i32)
}

#[macro_export]
macro_rules! exit {
    ($status:expr $(,)?) => {{
        $crate::syscall::exit::do_exit($status as u16)
    }};
}
