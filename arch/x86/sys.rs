use core::arch::asm;

pub fn call(num: u32, a: usize, b: usize, c: usize) -> usize {
    let ret: usize;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("eax") num as usize => ret,
            in("ebx") a,
            in("ecx") b,
            in("edx") c,
            options(nostack),
        );
    }
    ret
}
