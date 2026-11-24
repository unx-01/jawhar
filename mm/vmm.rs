pub use crate::pmm::PAGE_SIZE;

#[derive(Clone, Copy)]
pub struct Flags {
    pub writable: bool,
    pub user: bool,
    pub executable: bool,
}

pub const KERNEL_RO: Flags = Flags {
    writable: false,
    user: false,
    executable: false,
};

pub const KERNEL_RX: Flags = Flags {
    writable: false,
    user: false,
    executable: true,
};

pub const KERNEL_RW: Flags = Flags {
    writable: true,
    user: false,
    executable: false,
};

pub const KERNEL_RWX: Flags = Flags {
    writable: true,
    user: false,
    executable: true,
};

pub const USER_RO: Flags = Flags {
    writable: false,
    user: true,
    executable: false,
};

pub const USER_RX: Flags = Flags {
    writable: false,
    user: true,
    executable: true,
};

pub const USER_RW: Flags = Flags {
    writable: true,
    user: true,
    executable: false,
};

pub const USER_RWX: Flags = Flags {
    writable: true,
    user: true,
    executable: true,
};

#[derive(Clone, Copy)]
pub struct AddressSpace {
    root: usize,
    user: bool,
}

impl AddressSpace {
    pub fn kernel() -> Self {
        Self {
            root: imp::read_root(),
            user: false,
        }
    }

    pub fn current() -> Self {
        Self::kernel()
    }

    pub fn new() -> Option<Self> {
        imp::new_address_space().map(|root| Self { root, user: true })
    }

    pub fn root(&self) -> usize {
        self.root
    }

    pub fn is_user(&self) -> bool {
        self.user
    }

    pub fn map(&self, virt: usize, phys: usize, flags: Flags) -> bool {
        imp::map(self.root, self.user, virt, phys, flags)
    }

    pub fn unmap(&self, virt: usize) -> Option<usize> {
        imp::unmap(self.root, self.user, virt)
    }

    pub fn translate(&self, virt: usize) -> Option<usize> {
        imp::translate(self.root, virt)
    }

    pub fn activate(&self) {
        imp::activate(self.root);
    }

    pub fn destroy(self) {
        if self.user {
            unsafe {
                imp::destroy(self.root);
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod imp {
    use super::{Flags, PAGE_SIZE};
    use crate::pmm;
    use core::arch::asm;
    use core::ptr;

    const PRESENT: u64 = 1 << 0;
    const WRITABLE: u64 = 1 << 1;
    const USER: u64 = 1 << 2;
    const HUGE: u64 = 1 << 7;
    const NX: u64 = 1 << 63;
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    const LOWER_FLAGS: u64 = 0x1FFF;
    const UPPER_FLAGS: u64 = 0xFFF0_0000_0000_0000;
    const USER_FIRST: usize = 1;
    const USER_LAST: usize = 256;

    fn alloc_table() -> Option<usize> {
        let table = pmm::alloc_page()?;
        unsafe {
            ptr::write_bytes(table as *mut u8, 0, PAGE_SIZE);
        }
        Some(table)
    }

    unsafe fn table_empty(table: usize) -> bool {
        let table = table as *const u64;
        for index in 0..512 {
            if ptr::read(table.add(index)) != 0 {
                return false;
            }
        }
        true
    }

    unsafe fn read_cr3() -> usize {
        let value: u64;
        asm!("mov {}, cr3", out(reg) value, options(nostack, preserves_flags));
        (value & ADDR_MASK) as usize
    }

    unsafe fn write_cr3(root: usize) {
        asm!("mov cr3, {}", in(reg) root as u64, options(nostack, preserves_flags));
    }

    unsafe fn flush(virt: usize) {
        asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
    }

    unsafe fn flush_all() {
        write_cr3(read_cr3());
    }

    unsafe fn read_msr(msr: u32) -> u64 {
        let low: u32;
        let high: u32;
        asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nostack, preserves_flags));
        (low as u64) | ((high as u64) << 32)
    }

    unsafe fn write_msr(msr: u32, value: u64) {
        asm!("wrmsr", in("ecx") msr, in("eax") value as u32, in("edx") (value >> 32) as u32, options(nostack, preserves_flags));
    }

    fn enable_nx() {
        const EFER: u32 = 0xC000_0080;
        const NXE: u64 = 1 << 11;
        unsafe {
            let value = read_msr(EFER);
            write_msr(EFER, value | NXE);
        }
    }

    unsafe fn split_huge(entry: *mut u64, level: u8) -> Option<usize> {
        let value = ptr::read(entry);
        if value & HUGE == 0 {
            return Some((value & ADDR_MASK) as usize);
        }

        let child = alloc_table()?;
        let table = child as *mut u64;
        let flags = (value & LOWER_FLAGS & !HUGE) | (value & UPPER_FLAGS);

        if level == 1 {
            let base = value & 0x000F_FFFF_C000_0000;
            for index in 0..512u64 {
                ptr::write(table.add(index as usize), base | (index << 21) | flags | HUGE);
            }
        } else if level == 2 {
            let base = value & 0x000F_FFFF_FFE0_0000;
            for index in 0..512u64 {
                ptr::write(table.add(index as usize), base | (index << 12) | flags);
            }
        } else {
            pmm::free_page(child);
            return None;
        }

        ptr::write(entry, child as u64 | PRESENT | WRITABLE | (value & USER));
        flush_all();
        Some(child)
    }

    unsafe fn ensure_table(entry: *mut u64, level: u8, flags: Flags) -> Option<usize> {
        let mut value = ptr::read(entry);
        if value & PRESENT == 0 {
            let table = alloc_table()?;
            let mut new_value = table as u64 | PRESENT | WRITABLE;
            if flags.user {
                new_value |= USER;
            }
            ptr::write(entry, new_value);
            return Some(table);
        }

        if value & HUGE != 0 {
            split_huge(entry, level)?;
            value = ptr::read(entry);
        }

        if flags.writable && value & WRITABLE == 0 {
            value |= WRITABLE;
            ptr::write(entry, value);
        }
        if flags.user && value & USER == 0 {
            value |= USER;
            ptr::write(entry, value);
        }

        Some((value & ADDR_MASK) as usize)
    }

    fn leaf_value(phys: usize, flags: Flags) -> u64 {
        let mut value = (phys as u64 & ADDR_MASK) | PRESENT;
        if flags.writable {
            value |= WRITABLE;
        }
        if flags.user {
            value |= USER;
        }
        if !flags.executable {
            value |= NX;
        }
        value
    }

    pub fn read_root() -> usize {
        unsafe { read_cr3() }
    }

    pub fn init() {
        enable_nx();

        unsafe {
            let Some(pml4_page) = pmm::alloc_page() else {
                return;
            };
            let Some(pdpt_page) = pmm::alloc_page() else {
                pmm::free_page(pml4_page);
                return;
            };
            let Some(pd_page) = pmm::alloc_page() else {
                pmm::free_page(pdpt_page);
                pmm::free_page(pml4_page);
                return;
            };

            ptr::write_bytes(pml4_page as *mut u8, 0, PAGE_SIZE);
            ptr::write_bytes(pdpt_page as *mut u8, 0, PAGE_SIZE);
            ptr::write_bytes(pd_page as *mut u8, 0, PAGE_SIZE);

            let pml4 = pml4_page as *mut u64;
            let pdpt = pdpt_page as *mut u64;
            let pd = pd_page as *mut u64;

            ptr::write(pml4, pdpt_page as u64 | PRESENT | WRITABLE);
            ptr::write(pdpt, pd_page as u64 | PRESENT | WRITABLE);

            for index in 0..512u64 {
                ptr::write(
                    pd.add(index as usize),
                    (index << 21) | PRESENT | WRITABLE | HUGE,
                );
            }

            write_cr3(pml4_page);
        }
    }

    pub fn new_address_space() -> Option<usize> {
        let root = alloc_table()?;
        unsafe {
            let current = read_cr3() as *const u64;
            let new = root as *mut u64;
            ptr::copy_nonoverlapping(current, new, 1);
            ptr::copy_nonoverlapping(current.add(256), new.add(256), 256);
        }
        Some(root)
    }

    pub fn activate(root: usize) {
        unsafe {
            write_cr3(root);
        }
    }

    pub unsafe fn destroy(root: usize) {
        let pml4 = root as *mut u64;
        for i0 in USER_FIRST..USER_LAST {
            let pml4e = ptr::read(pml4.add(i0));
            if pml4e & PRESENT == 0 || pml4e & HUGE != 0 {
                continue;
            }
            let pdpt = (pml4e & ADDR_MASK) as *mut u64;
            for i1 in 0..512 {
                let pdpte = ptr::read(pdpt.add(i1));
                if pdpte & PRESENT == 0 || pdpte & HUGE != 0 {
                    continue;
                }
                let pd = (pdpte & ADDR_MASK) as *mut u64;
                for i2 in 0..512 {
                    let pde = ptr::read(pd.add(i2));
                    if pde & PRESENT == 0 || pde & HUGE != 0 {
                        continue;
                    }
                    pmm::free_page((pde & ADDR_MASK) as usize);
                }
                pmm::free_page(pd as usize);
            }
            pmm::free_page(pdpt as usize);
        }
        pmm::free_page(root);
    }

    pub fn map(root: usize, user_space: bool, virt: usize, phys: usize, flags: Flags) -> bool {
        if virt & (PAGE_SIZE - 1) != 0 || phys & (PAGE_SIZE - 1) != 0 {
            return false;
        }

        let i0 = (virt >> 39) & 0x1FF;
        if user_space && !(USER_FIRST..USER_LAST).contains(&i0) {
            return false;
        }

        unsafe {
            let pml4 = root as *mut u64;
            let Some(pdpt) = ensure_table(pml4.add(i0), 0, flags) else {
                return false;
            };
            let pdpt = pdpt as *mut u64;
            let Some(pd) = ensure_table(pdpt.add((virt >> 30) & 0x1FF), 1, flags) else {
                return false;
            };
            let pd = pd as *mut u64;
            let Some(pt) = ensure_table(pd.add((virt >> 21) & 0x1FF), 2, flags) else {
                return false;
            };
            let pt = pt as *mut u64;
            let entry = pt.add((virt >> 12) & 0x1FF);
            if ptr::read(entry) & PRESENT != 0 {
                return false;
            }
            ptr::write(entry, leaf_value(phys, flags));
            flush(virt);
        }

        true
    }

    pub fn unmap(root: usize, user_space: bool, virt: usize) -> Option<usize> {
        unsafe {
            let pml4 = root as *mut u64;
            let entry0 = pml4.add((virt >> 39) & 0x1FF);
            let value0 = ptr::read(entry0);
            if value0 & PRESENT == 0 || value0 & HUGE != 0 {
                return None;
            }
            let pdpt = (value0 & ADDR_MASK) as *mut u64;

            let entry1 = pdpt.add((virt >> 30) & 0x1FF);
            let value1 = ptr::read(entry1);
            if value1 & PRESENT == 0 {
                return None;
            }
            let pd = if value1 & HUGE != 0 {
                split_huge(entry1, 1)? as *mut u64
            } else {
                (value1 & ADDR_MASK) as *mut u64
            };

            let entry2 = pd.add((virt >> 21) & 0x1FF);
            let value2 = ptr::read(entry2);
            if value2 & PRESENT == 0 {
                return None;
            }
            let pt = if value2 & HUGE != 0 {
                split_huge(entry2, 2)? as *mut u64
            } else {
                (value2 & ADDR_MASK) as *mut u64
            };

            let entry3 = pt.add((virt >> 12) & 0x1FF);
            let value3 = ptr::read(entry3);
            if value3 & PRESENT == 0 {
                return None;
            }

            ptr::write(entry3, 0);
            flush(virt);

            if user_space && table_empty(pt as usize) {
                ptr::write(entry2, 0);
                pmm::free_page(pt as usize);
                if table_empty(pd as usize) {
                    ptr::write(entry1, 0);
                    pmm::free_page(pd as usize);
                    if table_empty(pdpt as usize) {
                        ptr::write(entry0, 0);
                        pmm::free_page(pdpt as usize);
                    }
                }
            }

            Some((value3 & ADDR_MASK) as usize)
        }
    }

    pub fn translate(root: usize, virt: usize) -> Option<usize> {
        unsafe {
            let pml4 = root as *const u64;
            let value0 = ptr::read(pml4.add((virt >> 39) & 0x1FF));
            if value0 & PRESENT == 0 || value0 & HUGE != 0 {
                return None;
            }
            let pdpt = (value0 & ADDR_MASK) as *const u64;
            let value1 = ptr::read(pdpt.add((virt >> 30) & 0x1FF));
            if value1 & PRESENT == 0 {
                return None;
            }
            if value1 & HUGE != 0 {
                let base = value1 & 0x000F_FFFF_C000_0000;
                return Some(base as usize + (virt & 0x3FFF_FFFF));
            }
            let pd = (value1 & ADDR_MASK) as *const u64;
            let value2 = ptr::read(pd.add((virt >> 21) & 0x1FF));
            if value2 & PRESENT == 0 {
                return None;
            }
            if value2 & HUGE != 0 {
                let base = value2 & 0x000F_FFFF_FFE0_0000;
                return Some(base as usize + (virt & 0x1F_FFFF));
            }
            let pt = (value2 & ADDR_MASK) as *const u64;
            let value3 = ptr::read(pt.add((virt >> 12) & 0x1FF));
            if value3 & PRESENT == 0 {
                return None;
            }
            Some((value3 & ADDR_MASK) as usize + (virt & (PAGE_SIZE - 1)))
        }
    }
}

#[cfg(target_arch = "x86")]
mod imp {
    use super::{Flags, PAGE_SIZE};
    use crate::pmm;
    use core::arch::asm;
    use core::ptr;

    const PRESENT: u32 = 1 << 0;
    const WRITABLE: u32 = 1 << 1;
    const USER: u32 = 1 << 2;
    const HUGE: u32 = 1 << 7;
    const ADDR_MASK: u32 = 0xFFFF_F000;
    const USER_FIRST: usize = 32;

    fn alloc_table() -> Option<usize> {
        let table = pmm::alloc_page()?;
        unsafe {
            ptr::write_bytes(table as *mut u8, 0, PAGE_SIZE);
        }
        Some(table)
    }

    unsafe fn table_empty(table: usize) -> bool {
        let table = table as *const u32;
        for index in 0..1024 {
            if ptr::read(table.add(index)) != 0 {
                return false;
            }
        }
        true
    }

    unsafe fn read_cr3() -> usize {
        let value: u32;
        asm!("mov {}, cr3", out(reg) value, options(nostack, preserves_flags));
        (value & ADDR_MASK) as usize
    }

    unsafe fn write_cr3(root: usize) {
        asm!("mov cr3, {}", in(reg) root as u32, options(nostack, preserves_flags));
    }

    unsafe fn flush(virt: usize) {
        asm!("invlpg [{}]", in(reg) virt as u32, options(nostack, preserves_flags));
    }

    unsafe fn flush_all() {
        write_cr3(read_cr3());
    }

    unsafe fn split_huge(entry: *mut u32) -> Option<usize> {
        let value = ptr::read(entry);
        if value & HUGE == 0 {
            return Some((value & ADDR_MASK) as usize);
        }

        let child = alloc_table()?;
        let table = child as *mut u32;
        let flags = value & 0xFFF & !HUGE;
        let base = value & 0xFFC0_0000;

        for index in 0..1024u32 {
            ptr::write(table.add(index as usize), base | (index << 12) | flags);
        }

        ptr::write(entry, child as u32 | PRESENT | WRITABLE | (value & USER));
        flush_all();
        Some(child)
    }

    unsafe fn ensure_table(entry: *mut u32, flags: Flags) -> Option<usize> {
        let mut value = ptr::read(entry);
        if value & PRESENT == 0 {
            let table = alloc_table()?;
            let mut new_value = table as u32 | PRESENT | WRITABLE;
            if flags.user {
                new_value |= USER;
            }
            ptr::write(entry, new_value);
            return Some(table);
        }

        if value & HUGE != 0 {
            split_huge(entry)?;
            value = ptr::read(entry);
        }

        if flags.writable && value & WRITABLE == 0 {
            value |= WRITABLE;
            ptr::write(entry, value);
        }
        if flags.user && value & USER == 0 {
            value |= USER;
            ptr::write(entry, value);
        }

        Some((value & ADDR_MASK) as usize)
    }

    fn leaf_value(phys: usize, flags: Flags) -> u32 {
        let mut value = (phys as u32 & ADDR_MASK) | PRESENT;
        if flags.writable {
            value |= WRITABLE;
        }
        if flags.user {
            value |= USER;
        }
        value
    }

    pub fn read_root() -> usize {
        unsafe { read_cr3() }
    }

    pub fn init() {
        unsafe {
            let Some(pd_page) = pmm::alloc_page() else {
                return;
            };
            ptr::write_bytes(pd_page as *mut u8, 0, PAGE_SIZE);
            let pd = pd_page as *mut u32;

            for index in 0..USER_FIRST {
                ptr::write(
                    pd.add(index),
                    (index as u32 * 0x40_0000) | PRESENT | WRITABLE | HUGE,
                );
            }

            let mut cr4: u32;
            asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
            cr4 |= 1 << 4;
            asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));

            write_cr3(pd_page);

            let mut cr0: u32;
            asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
            cr0 |= 1 << 31;
            asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));
        }
    }

    pub fn new_address_space() -> Option<usize> {
        let root = alloc_table()?;
        unsafe {
            let current = read_cr3() as *const u32;
            ptr::copy_nonoverlapping(current, root as *mut u32, USER_FIRST);
        }
        Some(root)
    }

    pub fn activate(root: usize) {
        unsafe {
            write_cr3(root);
        }
    }

    pub unsafe fn destroy(root: usize) {
        let pd = root as *mut u32;
        for index in USER_FIRST..1024 {
            let pde = ptr::read(pd.add(index));
            if pde & PRESENT == 0 || pde & HUGE != 0 {
                continue;
            }
            pmm::free_page((pde & ADDR_MASK) as usize);
        }
        pmm::free_page(root);
    }

    pub fn map(root: usize, user_space: bool, virt: usize, phys: usize, flags: Flags) -> bool {
        if virt & (PAGE_SIZE - 1) != 0 || phys & (PAGE_SIZE - 1) != 0 {
            return false;
        }

        let index = (virt >> 22) & 0x3FF;
        if user_space && index < USER_FIRST {
            return false;
        }

        unsafe {
            let pd = root as *mut u32;
            let Some(pt) = ensure_table(pd.add(index), flags) else {
                return false;
            };
            let pt = pt as *mut u32;
            let entry = pt.add((virt >> 12) & 0x3FF);
            if ptr::read(entry) & PRESENT != 0 {
                return false;
            }
            ptr::write(entry, leaf_value(phys, flags));
            flush(virt);
        }

        true
    }

    pub fn unmap(root: usize, user_space: bool, virt: usize) -> Option<usize> {
        unsafe {
            let pd = root as *mut u32;
            let index = (virt >> 22) & 0x3FF;
            let entry0 = pd.add(index);
            let value0 = ptr::read(entry0);
            if value0 & PRESENT == 0 {
                return None;
            }
            let pt = if value0 & HUGE != 0 {
                split_huge(entry0)? as *mut u32
            } else {
                (value0 & ADDR_MASK) as *mut u32
            };

            let entry1 = pt.add((virt >> 12) & 0x3FF);
            let value1 = ptr::read(entry1);
            if value1 & PRESENT == 0 {
                return None;
            }

            ptr::write(entry1, 0);
            flush(virt);

            if user_space && table_empty(pt as usize) {
                ptr::write(entry0, 0);
                pmm::free_page(pt as usize);
            }

            Some((value1 & ADDR_MASK) as usize)
        }
    }

    pub fn translate(root: usize, virt: usize) -> Option<usize> {
        unsafe {
            let pd = root as *const u32;
            let value0 = ptr::read(pd.add((virt >> 22) & 0x3FF));
            if value0 & PRESENT == 0 {
                return None;
            }
            if value0 & HUGE != 0 {
                let base = value0 & 0xFFC0_0000;
                return Some(base as usize + (virt & 0x3F_FFFF));
            }
            let pt = (value0 & ADDR_MASK) as *const u32;
            let value1 = ptr::read(pt.add((virt >> 12) & 0x3FF));
            if value1 & PRESENT == 0 {
                return None;
            }
            Some((value1 & ADDR_MASK) as usize + (virt & (PAGE_SIZE - 1)))
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod imp {
    use super::{Flags, PAGE_SIZE};
    use crate::pmm;
    use core::arch::asm;
    use core::ptr;

    const VALID: u64 = 1 << 0;
    const TABLE: u64 = 0b11;
    const BLOCK: u64 = 0b01;
    const AF: u64 = 1 << 10;
    const SH_INNER: u64 = 0b11 << 8;
    const ATTR_NORMAL: u64 = 0 << 2;
    const ATTR_DEVICE: u64 = 1 << 2;
    const AP_KERNEL_RW: u64 = 0 << 6;
    const AP_USER_RW: u64 = 1 << 6;
    const AP_KERNEL_RO: u64 = 2 << 6;
    const AP_USER_RO: u64 = 3 << 6;
    const PXN: u64 = 1 << 53;
    const UXN: u64 = 1 << 54;
    const CONTIGUOUS: u64 = 1 << 52;
    const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;
    const ATTR_FLAGS: u64 = 0xFFC;
    const USER_FIRST: usize = 1;

    fn alloc_table() -> Option<usize> {
        let table = pmm::alloc_page()?;
        unsafe {
            ptr::write_bytes(table as *mut u8, 0, PAGE_SIZE);
        }
        Some(table)
    }

    unsafe fn table_empty(table: usize) -> bool {
        let table = table as *const u64;
        for index in 0..512 {
            if ptr::read(table.add(index)) != 0 {
                return false;
            }
        }
        true
    }

    unsafe fn read_ttbr0() -> usize {
        let value: u64;
        asm!("mrs {}, TTBR0_EL1", out(reg) value, options(nostack));
        (value & ADDR_MASK) as usize
    }

    unsafe fn write_ttbr0(root: usize) {
        asm!("msr TTBR0_EL1, {}", in(reg) root as u64, options(nostack));
    }

    unsafe fn flush_all() {
        asm!(
            "dsb ishst",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            options(nostack)
        );
    }

    unsafe fn split_block(entry: *mut u64, level: u8) -> Option<usize> {
        let value = ptr::read(entry);
        if value & VALID == 0 || value & 0b10 != 0 {
            return Some((value & ADDR_MASK) as usize);
        }

        let child = alloc_table()?;
        let table = child as *mut u64;
        let flags = (value & ATTR_FLAGS) | (value & (PXN | UXN | CONTIGUOUS));

        if level == 1 {
            let base = value & 0x0000_FFFF_C000_0000;
            for index in 0..512u64 {
                ptr::write(
                    table.add(index as usize),
                    base | (index << 21) | flags | BLOCK,
                );
            }
        } else if level == 2 {
            let base = value & 0x0000_FFFF_FFE0_0000;
            for index in 0..512u64 {
                ptr::write(
                    table.add(index as usize),
                    base | (index << 12) | flags | TABLE,
                );
            }
        } else {
            pmm::free_page(child);
            return None;
        }

        ptr::write(entry, child as u64 | TABLE);
        flush_all();
        Some(child)
    }

    unsafe fn ensure_table(entry: *mut u64, level: u8) -> Option<usize> {
        let value = ptr::read(entry);
        if value & VALID == 0 {
            let table = alloc_table()?;
            ptr::write(entry, table as u64 | TABLE);
            return Some(table);
        }

        if value & 0b10 == 0 {
            return split_block(entry, level);
        }

        Some((value & ADDR_MASK) as usize)
    }

    fn leaf_value(phys: usize, flags: Flags) -> u64 {
        let mut value = (phys as u64 & ADDR_MASK) | TABLE | ATTR_NORMAL | SH_INNER | AF;

        if flags.user {
            value |= if flags.writable { AP_USER_RW } else { AP_USER_RO };
        } else {
            value |= if flags.writable { AP_KERNEL_RW } else { AP_KERNEL_RO };
        }

        if !flags.executable {
            value |= PXN | UXN;
        }

        value
    }

    pub fn read_root() -> usize {
        unsafe { read_ttbr0() }
    }

    pub fn init() {
        unsafe {
            let Some(l0_page) = pmm::alloc_page() else {
                return;
            };
            let Some(l1_page) = pmm::alloc_page() else {
                pmm::free_page(l0_page);
                return;
            };

            ptr::write_bytes(l0_page as *mut u8, 0, PAGE_SIZE);
            ptr::write_bytes(l1_page as *mut u8, 0, PAGE_SIZE);

            let l0 = l0_page as *mut u64;
            let l1 = l1_page as *mut u64;

            ptr::write(l1, ATTR_DEVICE | BLOCK | AF | PXN | UXN);
            ptr::write(
                l1.add(1),
                0x4000_0000 | ATTR_NORMAL | SH_INNER | BLOCK | AF | UXN,
            );
            ptr::write(l0, l1_page as u64 | TABLE);

            const MAIR: u64 = 0xFF;
            const TCR: u64 = (16)
                | (1 << 8)
                | (1 << 10)
                | (0b11 << 12)
                | (0 << 14)
                | (16 << 16)
                | (1 << 23)
                | (1 << 24)
                | (1 << 26)
                | (0b11 << 28)
                | (0b10 << 30)
                | (0b10 << 32);

            asm!("msr MAIR_EL1, {}", in(reg) MAIR, options(nostack));
            asm!("msr TCR_EL1, {}", in(reg) TCR, options(nostack));
            asm!("msr TTBR1_EL1, {}", in(reg) 0u64, options(nostack));
            write_ttbr0(l0_page);
            flush_all();

            let mut sctlr: u64;
            asm!("mrs {}, SCTLR_EL1", out(reg) sctlr, options(nostack));
            sctlr |= (1 << 0) | (1 << 2) | (1 << 12);
            asm!("msr SCTLR_EL1, {}", in(reg) sctlr, options(nostack));
            asm!("isb", options(nostack));
        }
    }

    pub fn new_address_space() -> Option<usize> {
        let root = alloc_table()?;
        unsafe {
            let current = read_ttbr0() as *const u64;
            let value = ptr::read(current);
            if value & VALID != 0 {
                ptr::write(root as *mut u64, value);
            }
        }
        Some(root)
    }

    pub fn activate(root: usize) {
        unsafe {
            write_ttbr0(root);
            flush_all();
        }
    }

    pub unsafe fn destroy(root: usize) {
        let l0 = root as *mut u64;
        for i0 in USER_FIRST..512 {
            let value0 = ptr::read(l0.add(i0));
            if value0 & VALID == 0 || value0 & 0b10 == 0 {
                continue;
            }
            let l1 = (value0 & ADDR_MASK) as *mut u64;
            for i1 in 0..512 {
                let value1 = ptr::read(l1.add(i1));
                if value1 & VALID == 0 || value1 & 0b10 == 0 {
                    continue;
                }
                let l2 = (value1 & ADDR_MASK) as *mut u64;
                for i2 in 0..512 {
                    let value2 = ptr::read(l2.add(i2));
                    if value2 & VALID == 0 || value2 & 0b10 == 0 {
                        continue;
                    }
                    pmm::free_page((value2 & ADDR_MASK) as usize);
                }
                pmm::free_page(l2 as usize);
            }
            pmm::free_page(l1 as usize);
        }
        pmm::free_page(root);
    }

    pub fn map(root: usize, user_space: bool, virt: usize, phys: usize, flags: Flags) -> bool {
        if virt & (PAGE_SIZE - 1) != 0 || phys & (PAGE_SIZE - 1) != 0 {
            return false;
        }

        let i0 = (virt >> 39) & 0x1FF;
        if user_space && i0 < USER_FIRST {
            return false;
        }

        unsafe {
            let l0 = root as *mut u64;
            let Some(l1) = ensure_table(l0.add(i0), 0) else {
                return false;
            };
            let l1 = l1 as *mut u64;
            let Some(l2) = ensure_table(l1.add((virt >> 30) & 0x1FF), 1) else {
                return false;
            };
            let l2 = l2 as *mut u64;
            let Some(l3) = ensure_table(l2.add((virt >> 21) & 0x1FF), 2) else {
                return false;
            };
            let l3 = l3 as *mut u64;
            let entry = l3.add((virt >> 12) & 0x1FF);
            if ptr::read(entry) & VALID != 0 {
                return false;
            }
            ptr::write(entry, leaf_value(phys, flags));
            flush_all();
        }

        true
    }

    pub fn unmap(root: usize, user_space: bool, virt: usize) -> Option<usize> {
        unsafe {
            let l0 = root as *mut u64;
            let entry0 = l0.add((virt >> 39) & 0x1FF);
            let value0 = ptr::read(entry0);
            if value0 & VALID == 0 || value0 & 0b10 == 0 {
                return None;
            }
            let l1 = (value0 & ADDR_MASK) as *mut u64;

            let entry1 = l1.add((virt >> 30) & 0x1FF);
            let value1 = ptr::read(entry1);
            if value1 & VALID == 0 {
                return None;
            }
            let l2 = if value1 & 0b10 == 0 {
                split_block(entry1, 1)? as *mut u64
            } else {
                (value1 & ADDR_MASK) as *mut u64
            };

            let entry2 = l2.add((virt >> 21) & 0x1FF);
            let value2 = ptr::read(entry2);
            if value2 & VALID == 0 {
                return None;
            }
            let l3 = if value2 & 0b10 == 0 {
                split_block(entry2, 2)? as *mut u64
            } else {
                (value2 & ADDR_MASK) as *mut u64
            };

            let entry3 = l3.add((virt >> 12) & 0x1FF);
            let value3 = ptr::read(entry3);
            if value3 & VALID == 0 {
                return None;
            }

            ptr::write(entry3, 0);
            flush_all();

            if user_space && table_empty(l3 as usize) {
                ptr::write(entry2, 0);
                pmm::free_page(l3 as usize);
                if table_empty(l2 as usize) {
                    ptr::write(entry1, 0);
                    pmm::free_page(l2 as usize);
                    if table_empty(l1 as usize) {
                        ptr::write(entry0, 0);
                        pmm::free_page(l1 as usize);
                    }
                }
            }

            Some((value3 & ADDR_MASK) as usize)
        }
    }

    pub fn translate(root: usize, virt: usize) -> Option<usize> {
        unsafe {
            let l0 = root as *const u64;
            let value0 = ptr::read(l0.add((virt >> 39) & 0x1FF));
            if value0 & VALID == 0 || value0 & 0b10 == 0 {
                return None;
            }
            let l1 = (value0 & ADDR_MASK) as *const u64;
            let value1 = ptr::read(l1.add((virt >> 30) & 0x1FF));
            if value1 & VALID == 0 {
                return None;
            }
            if value1 & 0b10 == 0 {
                let base = value1 & 0x0000_FFFF_C000_0000;
                return Some(base as usize + (virt & 0x3FFF_FFFF));
            }
            let l2 = (value1 & ADDR_MASK) as *const u64;
            let value2 = ptr::read(l2.add((virt >> 21) & 0x1FF));
            if value2 & VALID == 0 {
                return None;
            }
            if value2 & 0b10 == 0 {
                let base = value2 & 0x0000_FFFF_FFE0_0000;
                return Some(base as usize + (virt & 0x1F_FFFF));
            }
            let l3 = (value2 & ADDR_MASK) as *const u64;
            let value3 = ptr::read(l3.add((virt >> 12) & 0x1FF));
            if value3 & VALID == 0 {
                return None;
            }
            Some((value3 & ADDR_MASK) as usize + (virt & (PAGE_SIZE - 1)))
        }
    }
}

pub fn vmm_init() {
    imp::init();
}

pub fn map_page(virt: usize, phys: usize, flags: Flags) -> bool {
    imp::map(imp::read_root(), false, virt, phys, flags)
}

pub fn unmap_page(virt: usize) -> Option<usize> {
    imp::unmap(imp::read_root(), false, virt)
}

pub fn translate(virt: usize) -> Option<usize> {
    imp::translate(imp::read_root(), virt)
}
