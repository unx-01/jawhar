#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::spinlock::SpinLock;
use crate::time::ClockT;

struct State {
    ticks: u64,
    last: u64,
}

struct MsState {
    ms: u64,
    last: u64,
}

struct UsState {
    us: u64,
    last: u64,
}

static STATE: SpinLock<State> = SpinLock::new(State { ticks: 0, last: 0 });
static MS_STATE: SpinLock<MsState> = SpinLock::new(MsState { ms: 0, last: 0 });
static US_STATE: SpinLock<UsState> = SpinLock::new(UsState { us: 0, last: 0 });
static WALL_BASE: AtomicU64 = AtomicU64::new(0);
static WALL_TICK_BASE: AtomicU64 = AtomicU64::new(0);
static WALL_VALID: AtomicBool = AtomicBool::new(false);

pub fn init() {
    imp::arch_init();
    let now = imp::raw_ticks();
    {
        let mut state = STATE.lock();
        state.ticks = 0;
        state.last = now;
    }
    {
        let mut ms_state = MS_STATE.lock();
        ms_state.ms = 0;
        ms_state.last = now;
    }
    {
        let mut us_state = US_STATE.lock();
        us_state.us = 0;
        us_state.last = now;
    }

    if let Some(seconds) = imp::firmware_seconds() {
        WALL_BASE.store(seconds as u64, Ordering::Relaxed);
        WALL_TICK_BASE.store(0, Ordering::Relaxed);
        WALL_VALID.store(true, Ordering::Release);
    }
}

pub fn wall_seconds() -> u64 {
    if !WALL_VALID.load(Ordering::Acquire) {
        return 0;
    }

    let base = WALL_BASE.load(Ordering::Relaxed);
    let now = ticks();
    if now < 0 {
        return base;
    }

    let elapsed = (now as u64)
        .saturating_sub(WALL_TICK_BASE.load(Ordering::Relaxed))
        / crate::time::HZ;
    base.saturating_add(elapsed)
}

pub fn ticks() -> ClockT {
    let now = imp::raw_ticks();
    let mut state = STATE.lock();
    let (advance, last) = imp::advance(now, state.last);
    state.last = last;
    state.ticks = state.ticks.saturating_add(advance);
    state.ticks as ClockT
}

pub fn cycles() -> u64 {
    imp::raw_ticks()
}

pub fn cycles_to_ms(cycles: u64) -> u64 {
    imp::cycles_to_ms(cycles)
}

pub fn millis() -> u64 {
    let now = imp::raw_ticks();
    let mut state = MS_STATE.lock();
    let (advance, last) = imp::advance_ms(now, state.last);
    state.last = last;
    state.ms = state.ms.saturating_add(advance);
    state.ms
}

pub fn cycles_to_us(cycles: u64) -> u64 {
    imp::cycles_to_us(cycles)
}

pub fn micros() -> u64 {
    let now = imp::raw_ticks();
    let mut state = US_STATE.lock();
    let (advance, last) = imp::advance_us(now, state.last);
    state.last = last;
    state.us = state.us.saturating_add(advance);
    state.us
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod imp {
    use core::sync::atomic::{AtomicU64, Ordering};

    use crate::io::{inb, outb};

    const PIT_DATA: u16 = 0x40;
    const PIT_COMMAND: u16 = 0x43;
    const PIT_DIVISOR: u16 = 11_932;
    const CMOS_INDEX: u16 = 0x70;
    const CMOS_DATA: u16 = 0x71;

    static CYCLES_PER_TICK: AtomicU64 = AtomicU64::new(1);

    #[cfg(target_arch = "x86")]
    #[inline]
    unsafe fn div32(dividend: u32, divisor: u32) -> u32 {
        let quot: u32;
        let rem: u32;
        core::arch::asm!(
            "div {divisor}",
            divisor = in(reg) divisor,
            inout("eax") dividend => quot,
            inout("edx") 0u32 => rem,
            options(nostack, preserves_flags)
        );
        let _ = rem;
        quot
    }

    #[inline]
    fn per_ms() -> u64 {
        #[cfg(target_arch = "x86")]
        {
            ((CYCLES_PER_TICK.load(Ordering::Relaxed) as u32 / 10) | 1) as u64
        }

        #[cfg(target_arch = "x86_64")]
        {
            (CYCLES_PER_TICK.load(Ordering::Relaxed) / 10).max(1)
        }
    }

    #[inline]
    fn per_us() -> u64 {
        #[cfg(target_arch = "x86")]
        {
            ((CYCLES_PER_TICK.load(Ordering::Relaxed) as u32 / 10 / 1000) | 1) as u64
        }

        #[cfg(target_arch = "x86_64")]
        {
            (CYCLES_PER_TICK.load(Ordering::Relaxed) / 10 / 1000).max(1)
        }
    }

    #[inline]
    fn rdtsc() -> u64 {
        let lo: u32;
        let hi: u32;
        unsafe {
            core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nostack));
        }
        ((hi as u64) << 32) | lo as u64
    }

    unsafe fn pit_read() -> u16 {
        outb(PIT_COMMAND, 0x00);
        let lo = inb(PIT_DATA) as u16;
        let hi = inb(PIT_DATA) as u16;
        lo | (hi << 8)
    }

    pub(super) fn arch_init() {
        unsafe {
            outb(PIT_COMMAND, 0x34);
            outb(PIT_DATA, PIT_DIVISOR as u8);
            outb(PIT_DATA, (PIT_DIVISOR >> 8) as u8);
        }

        let mut prev = unsafe { pit_read() };
        loop {
            let current = unsafe { pit_read() };
            if current > prev {
                break;
            }
            prev = current;
            core::hint::spin_loop();
        }

        let start = rdtsc();
        prev = unsafe { pit_read() };
        let mut wraps = 0u64;
        while wraps < 8 {
            let current = unsafe { pit_read() };
            if current > prev {
                wraps += 1;
            }
            prev = current;
            core::hint::spin_loop();
        }
        let end = rdtsc();

        let cycles = if end > start {
            ((end - start) as u32 / 8) as u64
        } else {
            PIT_DIVISOR as u64
        };
        CYCLES_PER_TICK.store(cycles.max(1), Ordering::Relaxed);
    }

    unsafe fn cmos_read(reg: u8) -> u8 {
        outb(CMOS_INDEX, reg | 0x80);
        inb(CMOS_DATA)
    }

    fn bcd(value: u8) -> u8 {
        ((value >> 4) * 10) + (value & 0x0F)
    }

    fn days_from_civil(year: u32, month: u32, day: u32) -> u32 {
        let y = if month <= 2 { year - 1 } else { year };
        let era = y / 400;
        let yoe = y - era * 400;
        let mp = if month > 2 { month - 3 } else { month + 9 };
        let doy = (153 * mp + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    unsafe fn cmos_time(regb: u8) -> u32 {
        let mut second = cmos_read(0x00);
        let mut minute = cmos_read(0x02);
        let mut hour = cmos_read(0x04);
        let _weekday = cmos_read(0x06);
        let mut day = cmos_read(0x07);
        let mut month = cmos_read(0x08);
        let mut year = cmos_read(0x09);

        let pm = hour & 0x80 != 0;
        hour &= 0x7F;

        if regb & 0x04 == 0 {
            second = bcd(second);
            minute = bcd(minute);
            hour = bcd(hour);
            day = bcd(day);
            month = bcd(month);
            year = bcd(year);
        }

        let hour = if regb & 0x02 == 0 {
            let mut value = hour;
            if value == 12 {
                value = 0;
            }
            if pm {
                value += 12;
            }
            value
        } else {
            hour
        };

        let year = 2000u32 + year as u32;
        let days = days_from_civil(year, month as u32, day as u32);
        days * 86_400 + hour as u32 * 3_600 + minute as u32 * 60 + second as u32
    }

    fn wait_uip() -> bool {
        for _ in 0..4096 {
            if unsafe { cmos_read(0x0A) } & 0x80 == 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    pub(super) fn firmware_seconds() -> Option<u32> {
        unsafe {
            if !wait_uip() {
                return None;
            }
            let regb = cmos_read(0x0B);
            let first = cmos_time(regb);
            if !wait_uip() {
                return None;
            }
            let second = cmos_time(regb);
            if first == second {
                Some(second)
            } else {
                None
            }
        }
    }

    pub(super) fn raw_ticks() -> u64 {
        rdtsc()
    }

    pub(super) fn advance(now: u64, last: u64) -> (u64, u64) {
        let per_tick = CYCLES_PER_TICK.load(Ordering::Relaxed).max(1);

        #[cfg(target_arch = "x86")]
        {
            let now32 = now as u32;
            let last32 = last as u32;
            let per32 = (per_tick as u32) | 1;
            let elapsed = now32.wrapping_sub(last32);
            let count = unsafe { div32(elapsed, per32) };
            (count as u64, now32 as u64)
        }

        #[cfg(target_arch = "x86_64")]
        {
            if now < last {
                return (0, last);
            }
            let elapsed = now - last;
            let count = elapsed / per_tick;
            (count, last + count * per_tick)
        }
    }

    pub(super) fn cycles_to_ms(cycles: u64) -> u64 {
        let per_ms = per_ms();

        #[cfg(target_arch = "x86")]
        {
            unsafe { div32(cycles as u32, per_ms as u32) as u64 }
        }

        #[cfg(target_arch = "x86_64")]
        {
            cycles / per_ms
        }
    }

    pub(super) fn advance_ms(now: u64, last: u64) -> (u64, u64) {
        let per_ms = per_ms();

        #[cfg(target_arch = "x86")]
        {
            let now32 = now as u32;
            let elapsed = now32.wrapping_sub(last as u32);
            let count = unsafe { div32(elapsed, per_ms as u32) };
            (count as u64, now32 as u64)
        }

        #[cfg(target_arch = "x86_64")]
        {
            if now < last {
                return (0, last);
            }
            let elapsed = now - last;
            let count = elapsed / per_ms;
            (count, last + count * per_ms)
        }
    }

    pub(super) fn cycles_to_us(cycles: u64) -> u64 {
        let per_us = per_us();

        #[cfg(target_arch = "x86")]
        {
            unsafe { div32(cycles as u32, per_us as u32) as u64 }
        }

        #[cfg(target_arch = "x86_64")]
        {
            cycles / per_us
        }
    }

    pub(super) fn advance_us(now: u64, last: u64) -> (u64, u64) {
        let per_us = per_us();

        #[cfg(target_arch = "x86")]
        {
            let now32 = now as u32;
            let elapsed = now32.wrapping_sub(last as u32);
            let count = unsafe { div32(elapsed, per_us as u32) };
            (count as u64, now32 as u64)
        }

        #[cfg(target_arch = "x86_64")]
        {
            if now < last {
                return (0, last);
            }
            let elapsed = now - last;
            let count = elapsed / per_us;
            (count, last + count * per_us)
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod imp {
    use core::sync::atomic::{AtomicU64, Ordering};

    const PL031_RTCDR: usize = 0x0901_0000;

    static FREQ: AtomicU64 = AtomicU64::new(1_000_000);

    pub(super) fn firmware_seconds() -> Option<u32> {
        let seconds = unsafe { core::ptr::read_volatile(PL031_RTCDR as *const u32) };
        if seconds == u32::MAX {
            None
        } else {
            Some(seconds)
        }
    }

    pub(super) fn arch_init() {
        let freq: u64;
        unsafe {
            core::arch::asm!("mrs {}, CNTFRQ_EL0", out(reg) freq, options(nostack));
        }
        if freq != 0 {
            FREQ.store(freq, Ordering::Relaxed);
        }
    }

    pub(super) fn raw_ticks() -> u64 {
        let count: u64;
        unsafe {
            core::arch::asm!("mrs {}, CNTVCT_EL0", out(reg) count, options(nostack));
        }
        count
    }

    pub(super) fn advance(now: u64, last: u64) -> (u64, u64) {
        let per_tick = (FREQ.load(Ordering::Relaxed) / crate::time::HZ).max(1);
        if now < last {
            return (0, last);
        }
        let elapsed = now - last;
        let count = elapsed / per_tick;
        (count, last + count * per_tick)
    }

    pub(super) fn cycles_to_ms(cycles: u64) -> u64 {
        let per_ms = (FREQ.load(Ordering::Relaxed) / 1000).max(1);
        cycles / per_ms
    }

    pub(super) fn advance_ms(now: u64, last: u64) -> (u64, u64) {
        let per_ms = (FREQ.load(Ordering::Relaxed) / 1000).max(1);
        if now < last {
            return (0, last);
        }
        let elapsed = now - last;
        let count = elapsed / per_ms;
        (count, last + count * per_ms)
    }

    pub(super) fn cycles_to_us(cycles: u64) -> u64 {
        let per_us = (FREQ.load(Ordering::Relaxed) / 1_000_000).max(1);
        cycles / per_us
    }

    pub(super) fn advance_us(now: u64, last: u64) -> (u64, u64) {
        let per_us = (FREQ.load(Ordering::Relaxed) / 1_000_000).max(1);
        if now < last {
            return (0, last);
        }
        let elapsed = now - last;
        let count = elapsed / per_us;
        (count, last + count * per_us)
    }
}
