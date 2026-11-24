#![allow(dead_code)]

pub type ClockT = isize;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Tms {
    pub tms_utime: ClockT,
    pub tms_stime: ClockT,
    pub tms_cutime: ClockT,
    pub tms_cstime: ClockT,
}

pub const HZ: u64 = 100;
pub const CLK_TCK: ClockT = HZ as ClockT;
pub const CLOCKS_PER_SEC: ClockT = HZ as ClockT;
