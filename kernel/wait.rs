pub fn do_wait(pid: i32, status: *mut i32) -> i32 {
    crate::process::wait(pid, status)
}

#[macro_export]
macro_rules! wait {
    ($pid:expr, $status:expr $(,)?) => {{
        $crate::syscall::wait::do_wait($pid as i32, $status)
    }};
}
