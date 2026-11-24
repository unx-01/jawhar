//! System-call entry points.
//!
//! `__syscall!` is the variadic entry point, modelled on Linux's
//! `long syscall(long number, ...)`. Stable Rust has no C-variadic functions,
//! so it is a macro that accepts zero to six arguments and pads the missing
//! ones with zero before calling the fixed-arity dispatcher.

/// Variadic system call: `__syscall!(number, arg1, arg2, ...)`.
#[macro_export]
macro_rules! __syscall {
    ($number:expr $(,)?) => {
        $crate::syscall::__syscall_raw($number as u32, 0, 0, 0, 0, 0)
    };
    ($number:expr, $a1:expr $(,)?) => {
        $crate::syscall::__syscall_raw($number as u32, $a1 as usize, 0, 0, 0, 0)
    };
    ($number:expr, $a1:expr, $a2:expr $(,)?) => {
        $crate::syscall::__syscall_raw(
            $number as u32,
            $a1 as usize,
            $a2 as usize,
            0,
            0,
            0,
        )
    };
    ($number:expr, $a1:expr, $a2:expr, $a3:expr $(,)?) => {
        $crate::syscall::__syscall_raw(
            $number as u32,
            $a1 as usize,
            $a2 as usize,
            $a3 as usize,
            0,
            0,
        )
    };
    ($number:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr $(,)?) => {
        $crate::syscall::__syscall_raw(
            $number as u32,
            $a1 as usize,
            $a2 as usize,
            $a3 as usize,
            $a4 as usize,
            0,
        )
    };
    ($number:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr $(,)?) => {
        $crate::syscall::__syscall_raw(
            $number as u32,
            $a1 as usize,
            $a2 as usize,
            $a3 as usize,
            $a4 as usize,
            $a5 as usize,
        )
    };
}

/// Alias for `__syscall!`, kept for existing callers.
#[macro_export]
macro_rules! syscall {
    ($($args:tt)*) => { $crate::__syscall!($($args)*) };
}
