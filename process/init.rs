use core::sync::atomic::{AtomicI32, Ordering};

static NEXT_PID: AtomicI32 = AtomicI32::new(1);

pub fn _exit(_status: i32) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

pub fn fork(_process: i32) -> i32 {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}

pub fn thread() {}
