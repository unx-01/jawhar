pub fn do_fork() -> i32 {
    crate::process::fork()
}

#[macro_export]
macro_rules! fork {
    () => {{
        $crate::syscall::fork::do_fork()
    }};
}
