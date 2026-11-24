use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const PAGE_SIZE: usize = 4096;

#[cfg(target_arch = "x86_64")]
pub const MAX_PHYS: usize = 1024 * 1024 * 1024;

#[cfg(target_arch = "x86")]
pub const MAX_PHYS: usize = 128 * 1024 * 1024;

#[cfg(target_arch = "aarch64")]
pub const MAX_PHYS: usize = 0x4800_0000;

const PAGE_COUNT: usize = MAX_PHYS / PAGE_SIZE;
const BITMAP_BYTES: usize = PAGE_COUNT / 8;
const LOW_MEMORY: usize = 0x10_0000;
const MULTIBOOT_MAGIC: u32 = 0x2BADB002;

#[cfg(target_arch = "aarch64")]
const RAM_START: usize = 0x4000_0000;

#[cfg(target_arch = "aarch64")]
const RAM_END: usize = 0x4800_0000;

static mut BITMAP: [u8; BITMAP_BYTES] = [0; BITMAP_BYTES];
static TOTAL_PAGES: AtomicUsize = AtomicUsize::new(0);
static FREE_PAGES: AtomicUsize = AtomicUsize::new(0);
static NEXT_HINT: AtomicUsize = AtomicUsize::new(0);

fn bitmap_ptr() -> *mut u8 {
    ptr::addr_of_mut!(BITMAP) as *mut u8
}

unsafe fn is_used(page: usize) -> bool {
    let byte = ptr::read(bitmap_ptr().add(page / 8));
    byte & (1 << (page % 8)) != 0
}

unsafe fn set_used(page: usize, used: bool) {
    let byte = bitmap_ptr().add(page / 8);
    let mut value = ptr::read(byte);
    let mask = 1u8 << (page % 8);
    if used {
        value |= mask;
    } else {
        value &= !mask;
    }
    ptr::write(byte, value);
}

unsafe fn free_region(start: usize, end: usize) {
    let mut start = (start.saturating_add(PAGE_SIZE - 1)) & !(PAGE_SIZE - 1);
    let end = end & !(PAGE_SIZE - 1);
    if start < LOW_MEMORY {
        start = LOW_MEMORY;
    }
    if start >= end || start >= MAX_PHYS {
        return;
    }
    let end = end.min(MAX_PHYS);
    let first = start / PAGE_SIZE;
    let last = end / PAGE_SIZE;

    for page in first..last {
        if is_used(page) {
            set_used(page, false);
            FREE_PAGES.fetch_add(1, Ordering::Relaxed);
            TOTAL_PAGES.fetch_add(1, Ordering::Relaxed);
        }
    }
    NEXT_HINT.store(first, Ordering::Relaxed);
}

pub unsafe fn mark_used(start: usize, size: usize) {
    let end = start.saturating_add(size);
    if start >= MAX_PHYS {
        return;
    }
    let first = start / PAGE_SIZE;
    let last = (end.min(MAX_PHYS).saturating_add(PAGE_SIZE - 1)) / PAGE_SIZE;

    for page in first..last {
        if !is_used(page) {
            set_used(page, true);
            FREE_PAGES.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub unsafe fn mark_free(start: usize, size: usize) {
    free_region(start, start.saturating_add(size));
}

unsafe fn reserve_kernel() {
    extern "C" {
        static __bss_end: u8;
    }
    let end = core::ptr::addr_of!(__bss_end) as usize;
    mark_used(0, end);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn multiboot_regions(magic: usize, info: usize) -> bool {
    if magic as u32 != MULTIBOOT_MAGIC || info == 0 {
        return false;
    }

    let flags = ptr::read_unaligned(info as *const u32);
    if flags & (1 << 6) == 0 {
        return false;
    }

    let length = ptr::read_unaligned((info + 44) as *const u32) as usize;
    let addr = ptr::read_unaligned((info + 48) as *const u32) as usize;
    if length == 0 || addr == 0 {
        return false;
    }

    let end = addr.saturating_add(length);
    let mut entry = addr;
    while entry.saturating_add(24) <= end {
        let size = ptr::read_unaligned(entry as *const u32) as usize;
        if size < 20 {
            break;
        }
        let base = ptr::read_unaligned((entry + 4) as *const u64) as usize;
        let len = ptr::read_unaligned((entry + 12) as *const u64) as usize;
        let kind = ptr::read_unaligned((entry + 20) as *const u32);
        if kind == 1 {
            free_region(base, base.saturating_add(len));
        }
        entry = entry.saturating_add(size + 4);
    }

    mark_used(addr, length);
    mark_used(info, PAGE_SIZE);
    true
}

pub fn pmm_init(magic: usize, info: usize) {
    unsafe {
        ptr::write_bytes(bitmap_ptr(), 0xFF, BITMAP_BYTES);
        TOTAL_PAGES.store(0, Ordering::Relaxed);
        FREE_PAGES.store(0, Ordering::Relaxed);
        NEXT_HINT.store(0, Ordering::Relaxed);

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if !multiboot_regions(magic, info) {
                free_region(LOW_MEMORY, 128 * 1024 * 1024);
                if info != 0 {
                    mark_used(info, PAGE_SIZE);
                }
            }
            reserve_kernel();
        }

        #[cfg(target_arch = "aarch64")]
        {
            let _ = (magic, info);
            free_region(RAM_START, RAM_END);
            reserve_kernel();
        }
    }
}

pub fn alloc_page() -> Option<usize> {
    unsafe {
        if FREE_PAGES.load(Ordering::Relaxed) == 0 {
            return None;
        }

        let mut page = NEXT_HINT.load(Ordering::Relaxed);
        loop {
            if page >= PAGE_COUNT {
                page = 0;
            }
            if !is_used(page) {
                set_used(page, true);
                FREE_PAGES.fetch_sub(1, Ordering::Relaxed);
                NEXT_HINT.store((page + 1) % PAGE_COUNT, Ordering::Relaxed);
                return Some(page * PAGE_SIZE);
            }
            page += 1;
        }
    }
}

pub fn alloc_pages(count: usize) -> Option<usize> {
    if count == 0 || FREE_PAGES.load(Ordering::Relaxed) < count {
        return None;
    }

    unsafe {
        let mut run = 0;
        let mut start = 0;
        for page in 0..PAGE_COUNT {
            if is_used(page) {
                run = 0;
                continue;
            }

            if run == 0 {
                start = page;
            }
            run += 1;
            if run == count {
                for allocated in start..start + count {
                    set_used(allocated, true);
                }
                FREE_PAGES.fetch_sub(count, Ordering::Relaxed);
                NEXT_HINT.store(start + count, Ordering::Relaxed);
                return Some(start * PAGE_SIZE);
            }
        }
    }

    None
}

pub fn free_page(addr: usize) {
    let page = addr / PAGE_SIZE;
    if page >= PAGE_COUNT {
        return;
    }

    unsafe {
        if is_used(page) {
            set_used(page, false);
            FREE_PAGES.fetch_add(1, Ordering::Relaxed);
            if page < NEXT_HINT.load(Ordering::Relaxed) {
                NEXT_HINT.store(page, Ordering::Relaxed);
            }
        }
    }
}

pub fn free_pages(addr: usize, count: usize) {
    for page in 0..count {
        free_page(addr + page * PAGE_SIZE);
    }
}

pub fn total_pages() -> usize {
    TOTAL_PAGES.load(Ordering::Relaxed)
}

pub fn free_page_count() -> usize {
    FREE_PAGES.load(Ordering::Relaxed)
}

pub fn used_pages() -> usize {
    total_pages().saturating_sub(free_page_count())
}
