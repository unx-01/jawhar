use core::arch::asm;

pub fn call(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "svc 0",
            in("x8") num,
            inout("x0") a1 => ret,
            in("x1") a2,
            in("x2") a3,
            options(nostack),
        );
    }
    ret
}
