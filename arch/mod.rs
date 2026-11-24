#![allow(dead_code)]

pub mod smp;
pub mod time;

#[cfg(target_arch = "x86_64")]
#[path = "x86-64/mod.rs"]
pub mod x86_64;

#[cfg(target_arch = "x86")]
pub mod x86;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "x86_64")]
use x86_64::smp as imp_smp;
#[cfg(target_arch = "x86")]
use x86::smp as imp_smp;
#[cfg(target_arch = "aarch64")]
use aarch64::smp as imp_smp;

/// يُستدعى من kernel_main() مرة واحدة في بداية النواة.
/// كل معمارية تنفّذ ما يناسبها:
///   - x86_64/x86 → gdt::init()
///   - aarch64    → mmu::init()  (لا يوجد GDT)
pub fn init() {
    #[cfg(target_arch = "x86_64")]
    {
        x86_64::init();
        enable_sse64();
    }

    #[cfg(target_arch = "x86")]
    {
        x86::init();
        enable_sse32();
    }

    #[cfg(target_arch = "aarch64")]
    aarch64::init();

    time::init();
    smp::init();
}

#[cfg(target_arch = "x86_64")]
fn enable_sse64() {
    unsafe {
        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= (1 << 9) | (1 << 10);
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));
    }
}

#[cfg(target_arch = "x86")]
fn enable_sse32() {
    unsafe {
        let mut cr4: u32;
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= (1 << 9) | (1 << 10);
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));
    }
}

pub fn halt_forever() -> ! {
    loop {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        unsafe { core::arch::asm!("cli; hlt", options(nostack)); }

        #[cfg(target_arch = "aarch64")]
        unsafe { core::arch::asm!("wfe", options(nostack)); }
    }
}
