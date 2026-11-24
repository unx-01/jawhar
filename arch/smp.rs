#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering};

pub const MAX_CPUS: usize = 8;

#[derive(Clone, Copy)]
pub struct CpuInfo {
    pub online: bool,
    pub apic_id: usize,
    pub hart_id: usize,
}

impl CpuInfo {
    pub const fn empty() -> Self {
        Self {
            online: false,
            apic_id: 0,
            hart_id: 0,
        }
    }
}

static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);
static ONLINE_CPUS: AtomicUsize = AtomicUsize::new(1);
static BSP_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_LOGICAL_CPU: AtomicUsize = AtomicUsize::new(1);
static mut CPUS: [CpuInfo; MAX_CPUS] = [CpuInfo::empty(); MAX_CPUS];

pub fn init() {
    unsafe {
        let cpus = core::ptr::addr_of_mut!(CPUS) as *mut CpuInfo;
        for i in 0..MAX_CPUS {
            core::ptr::write(cpus.add(i), CpuInfo::empty());
        }
        (*cpus).online = true;
    }
    CPU_COUNT.store(1, Ordering::Relaxed);
    ONLINE_CPUS.store(1, Ordering::Relaxed);
    NEXT_LOGICAL_CPU.store(1, Ordering::Relaxed);

    super::imp_smp::init_bsp();
}

pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

pub fn set_cpu_count(count: usize) {
    let count = if count == 0 { 1 } else if count > MAX_CPUS { MAX_CPUS } else { count };
    CPU_COUNT.store(count, Ordering::Relaxed);
}

pub fn online_cpus() -> usize {
    ONLINE_CPUS.load(Ordering::Relaxed)
}

pub fn bsp_id() -> usize {
    BSP_ID.load(Ordering::Relaxed)
}

pub fn cpu_id() -> usize {
    super::imp_smp::cpu_id()
}

pub fn alloc_logical_cpu() -> Option<usize> {
    loop {
        let id = NEXT_LOGICAL_CPU.load(Ordering::Relaxed);
        if id >= MAX_CPUS {
            return None;
        }
        if NEXT_LOGICAL_CPU
            .compare_exchange(id, id + 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            return Some(id);
        }
    }
}

pub fn mark_online(id: usize) {
    if id >= MAX_CPUS {
        return;
    }
    unsafe {
        let cpu = (core::ptr::addr_of_mut!(CPUS) as *mut CpuInfo).add(id);
        if !(*cpu).online {
            (*cpu).online = true;
            ONLINE_CPUS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn set_cpu_info(id: usize, apic_id: usize, hart_id: usize) {
    if id >= MAX_CPUS {
        return;
    }
    unsafe {
        let cpu = (core::ptr::addr_of_mut!(CPUS) as *mut CpuInfo).add(id);
        (*cpu).apic_id = apic_id;
        (*cpu).hart_id = hart_id;
    }
}

pub fn cpu_info(id: usize) -> Option<CpuInfo> {
    if id >= MAX_CPUS {
        return None;
    }
    unsafe { Some(*((core::ptr::addr_of!(CPUS) as *const CpuInfo).add(id))) }
}

pub fn is_bsp() -> bool {
    cpu_id() == bsp_id()
}
