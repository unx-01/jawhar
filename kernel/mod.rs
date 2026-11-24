#![allow(dead_code)]

use crate::time;

pub mod clock;
pub mod exit;
pub mod fork;
pub mod read;
pub mod syscall;
pub mod times;
pub mod wait;
pub mod write;

pub const SYS_SYSCALL: u32 = 0;
pub const SYS_WRITE: u32 = 1;
pub const SYS_READ: u32 = 2;
pub const SYS_OPEN: u32 = 3;
pub const SYS_CLOSE: u32 = 4;
pub const SYS_FORK: u32 = 6;
pub const SYS_EXIT: u32 = 7;
pub const SYS_WAIT4: u32 = 9;
pub const SYS_TIMES: u32 = 10;
pub const SYS_CLOCK: u32 = 11;

/// Fixed-arity dispatcher behind the variadic `__syscall!` macro.
///
/// Callers should use `__syscall!(number, ...)`, which mirrors Linux's
/// `syscall(long number, ...)` and pads omitted arguments with zero.
pub fn __syscall_raw(
    number: u32,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> isize {
    match number {
        SYS_WRITE => write::do_write(arg1 as i32, arg2 as *const u8, arg3 as usize),
        SYS_FORK => fork::do_fork() as isize,
        SYS_EXIT => exit::do_exit(arg1 as u16),
        SYS_WAIT4 => wait::do_wait(arg1 as i32, arg2 as *mut i32) as isize,
        SYS_TIMES => times::do_times(arg1 as *mut time::Tms),
        SYS_CLOCK => clock::do_clock(),
        SYS_READ | SYS_OPEN | SYS_CLOSE | SYS_SYSCALL => -1,
        _ => -1,
    }
}
