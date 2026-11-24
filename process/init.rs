use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use crate::time::{ClockT, Tms};

const MAX_PROC: usize = 16;
const STACK_PAGES: usize = 16;
const STACK_SIZE: usize = STACK_PAGES * crate::pmm::PAGE_SIZE;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    sp: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unused,
    Ready,
    Running,
    Zombie,
}

#[derive(Clone, Copy)]
struct Process {
    pid: i32,
    ppid: i32,
    state: State,
    context: Context,
    stack_low: usize,
    stack_high: usize,
    stack_owned: bool,
    exit_status: i32,
    utime: ClockT,
    stime: ClockT,
    cutime: ClockT,
    cstime: ClockT,
    last_tick: ClockT,
}

impl Process {
    const EMPTY: Process = Process {
        pid: 0,
        ppid: 0,
        state: State::Unused,
        context: Context { sp: 0 },
        stack_low: 0,
        stack_high: 0,
        stack_owned: false,
        exit_status: 0,
        utime: 0,
        stime: 0,
        cutime: 0,
        cstime: 0,
        last_tick: 0,
    };
}

static mut PROCS: [Process; MAX_PROC] = [Process::EMPTY; MAX_PROC];
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static NEXT_PID: AtomicI32 = AtomicI32::new(1);

extern "C" {
    fn fork_save(
        parent: *mut Context,
        child: *mut Context,
        old_low: usize,
        old_high: usize,
        new_low: usize,
        new_high: usize,
    ) -> usize;
    fn context_switch(old: *mut Context, new: *const Context);
}

fn procs_ptr() -> *mut Process {
    core::ptr::addr_of_mut!(PROCS) as *mut Process
}

fn proc_ptr(index: usize) -> *mut Process {
    unsafe { procs_ptr().add(index) }
}

fn current_ptr() -> *mut Process {
    proc_ptr(CURRENT.load(Ordering::Relaxed))
}

pub fn init() {
    extern "C" {
        static stack_bottom: u8;
        static stack_top: u8;
    }

    unsafe {
        let low = core::ptr::addr_of!(stack_bottom) as usize;
        let high = core::ptr::addr_of!(stack_top) as usize;
        let base = procs_ptr();
        for index in 0..MAX_PROC {
            core::ptr::write(base.add(index), Process::EMPTY);
        }

        let process = proc_ptr(0);
        (*process).pid = 1;
        (*process).ppid = 0;
        (*process).state = State::Running;
        (*process).stack_low = low;
        (*process).stack_high = high;
        (*process).stack_owned = false;
        (*process).last_tick = crate::arch::time::ticks();

        CURRENT.store(0, Ordering::Relaxed);
        NEXT_PID.store(2, Ordering::Relaxed);
    }
}

unsafe fn alloc_slot() -> Option<usize> {
    for index in 0..MAX_PROC {
        if (*proc_ptr(index)).state == State::Unused {
            return Some(index);
        }
    }
    None
}

#[no_mangle]
pub unsafe extern "C" fn fork_copy(
    saved_sp: usize,
    old_low: usize,
    old_high: usize,
    new_low: usize,
    new_high: usize,
    child: *mut Context,
) -> usize {
    if saved_sp < old_low || saved_sp >= old_high {
        return 0;
    }

    let used = old_high - saved_sp;
    if new_high < new_low {
        return 0;
    }
    let span = new_high - new_low;
    if used.saturating_add(crate::pmm::PAGE_SIZE) > span {
        return 0;
    }

    let new_sp = new_high - used;
    core::ptr::copy_nonoverlapping(saved_sp as *const u8, new_sp as *mut u8, used);

    let word = core::mem::size_of::<usize>();
    let mut offset = 0;
    while offset + word <= used {
        let slot = (new_sp + offset) as *mut usize;
        let value = core::ptr::read(slot);
        if value >= old_low && value <= old_high {
            let distance = old_high - value;
            if distance <= new_high {
                let mapped = new_high - distance;
                if mapped >= new_low {
                    core::ptr::write(slot, mapped);
                }
            }
        }
        offset += word;
    }

    if !child.is_null() {
        (*child).sp = new_sp;
    }
    1
}

pub fn fork() -> i32 {
    account_current();
    unsafe {
        let parent_index = CURRENT.load(Ordering::Relaxed);
        let parent = &mut *proc_ptr(parent_index);
        let Some(child_index) = alloc_slot() else {
            return -1;
        };
        let Some(stack) = crate::pmm::alloc_pages(STACK_PAGES) else {
            return -1;
        };

        let child = &mut *proc_ptr(child_index);
        core::ptr::write(child, Process::EMPTY);
        child.pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        child.ppid = parent.pid;
        child.state = State::Ready;
        child.stack_low = stack;
        child.stack_high = stack + STACK_SIZE;
        child.stack_owned = true;
        child.last_tick = crate::arch::time::ticks();

        let ok = fork_save(
            &mut parent.context as *mut Context,
            &mut child.context as *mut Context,
            parent.stack_low,
            parent.stack_high,
            child.stack_low,
            child.stack_high,
        );

        if CURRENT.load(Ordering::Relaxed) != parent_index {
            return 0;
        }

        if ok == 0 {
            child.state = State::Unused;
            child.pid = 0;
            child.ppid = 0;
            child.stack_low = 0;
            child.stack_high = 0;
            child.stack_owned = false;
            crate::pmm::free_pages(stack, STACK_PAGES);
            return -1;
        }

        child.pid
    }
}

unsafe fn pick_next(start: usize) -> usize {
    let mut step = 1;
    while step <= MAX_PROC {
        let index = (start + step) % MAX_PROC;
        let state = (*proc_ptr(index)).state;
        if state == State::Ready || state == State::Running {
            return index;
        }
        step += 1;
    }
    start
}

unsafe fn switch_to_next() {
    account_current();
    let current = CURRENT.load(Ordering::Relaxed);
    let next = pick_next(current);
    if next == current {
        if (*proc_ptr(current)).state == State::Ready {
            (*proc_ptr(current)).state = State::Running;
        }
        return;
    }

    let old = proc_ptr(current);
    let new = proc_ptr(next);
    if (*old).state == State::Running {
        (*old).state = State::Ready;
    }
    (*new).state = State::Running;
    CURRENT.store(next, Ordering::Relaxed);
    context_switch(
        core::ptr::addr_of_mut!((*old).context),
        core::ptr::addr_of!((*new).context),
    );
}

pub fn yield_now() {
    unsafe {
        switch_to_next();
    }
}

pub fn account_current() {
    let now = crate::arch::time::ticks();
    unsafe {
        let current = current_ptr();
        if now > (*current).last_tick {
            let delta = now - (*current).last_tick;
            (*current).utime = (*current).utime.saturating_add(delta);
        }
        (*current).last_tick = now;
    }
}

pub fn times(buf: *mut Tms) -> ClockT {
    account_current();
    let now = crate::arch::time::ticks();
    unsafe {
        if !buf.is_null() {
            let current = &*current_ptr();
            *buf = Tms {
                tms_utime: current.utime,
                tms_stime: current.stime,
                tms_cutime: current.cutime,
                tms_cstime: current.cstime,
            };
        }
    }
    now
}

pub fn clock() -> ClockT {
    account_current();
    unsafe {
        let current = &*current_ptr();
        current.utime.saturating_add(current.stime)
    }
}

pub fn exit(status: i32) -> ! {
    account_current();
    unsafe {
        let current = current_ptr();
        let pid = (*current).pid;
        for child_index in 0..MAX_PROC {
            let child = proc_ptr(child_index);
            if child == current || (*child).state == State::Unused {
                continue;
            }
            if (*child).ppid == pid {
                (*child).ppid = 1;
            }
        }

        (*current).state = State::Zombie;
        (*current).exit_status = status;

        let index = CURRENT.load(Ordering::Relaxed);
        let next = pick_next(index);
        if next == index {
            crate::arch::halt_forever();
        }

        let new = proc_ptr(next);
        (*new).state = State::Running;
        CURRENT.store(next, Ordering::Relaxed);
        context_switch(
            core::ptr::addr_of_mut!((*current).context),
            core::ptr::addr_of!((*new).context),
        );
        crate::arch::halt_forever();
    }
}

pub fn wait(pid: i32, status: *mut i32) -> i32 {
    unsafe {
        let parent_ptr = current_ptr();
        let parent = (*parent_ptr).pid;
        loop {
            let mut waiting = false;
            for index in 0..MAX_PROC {
                let child = proc_ptr(index);
                if (*child).state == State::Unused || (*child).ppid != parent {
                    continue;
                }
                if pid != -1 && pid != (*child).pid {
                    continue;
                }
                waiting = true;
                if (*child).state == State::Zombie {
                    let child_pid = (*child).pid;
                    let child_status = (*child).exit_status;
                    let child_utime = (*child).utime;
                    let child_stime = (*child).stime;
                    let stack = (*child).stack_low;
                    let owned = (*child).stack_owned;

                    (*parent_ptr).cutime = (*parent_ptr).cutime.saturating_add(child_utime);
                    (*parent_ptr).cstime = (*parent_ptr).cstime.saturating_add(child_stime);

                    (*child).state = State::Unused;
                    (*child).pid = 0;
                    (*child).ppid = 0;
                    (*child).stack_low = 0;
                    (*child).stack_high = 0;
                    (*child).stack_owned = false;

                    if owned {
                        crate::pmm::free_pages(stack, STACK_PAGES);
                    }
                    if !status.is_null() {
                        *status = child_status;
                    }
                    return child_pid;
                }
            }

            if !waiting {
                return -1;
            }
            switch_to_next();
        }
    }
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .text
    .global fork_save
fork_save:
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov [rdi], rsp
    mov r10, rsi
    mov r11, rdx
    mov rdi, rsp
    mov rsi, r11
    mov rdx, rcx
    mov rcx, r8
    mov r8, r9
    mov r9, r10
    sub rsp, 8
    call fork_copy
    add rsp, 8
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret

    .global context_switch
context_switch:
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov [rdi], rsp
    mov rsp, [rsi]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret
    "#
);

#[cfg(target_arch = "x86")]
core::arch::global_asm!(
    r#"
    .text
    .global fork_save
fork_save:
    push ebx
    push ebp
    push esi
    push edi
    mov eax, esp
    mov ecx, [esp + 20]
    mov [ecx], eax
    mov ebx, [esp + 24]
    mov ecx, [esp + 28]
    mov edx, [esp + 32]
    mov esi, [esp + 36]
    mov edi, [esp + 40]
    sub esp, 12
    push ebx
    push edi
    push esi
    push edx
    push ecx
    push eax
    call fork_copy
    add esp, 36
    pop edi
    pop esi
    pop ebp
    pop ebx
    ret

    .global context_switch
context_switch:
    push ebx
    push ebp
    push esi
    push edi
    mov eax, [esp + 20]
    mov [eax], esp
    mov eax, [esp + 24]
    mov esp, [eax]
    pop edi
    pop esi
    pop ebp
    pop ebx
    ret
    "#
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .text
    .global fork_save
fork_save:
    sub sp, sp, #96
    stp x19, x20, [sp, #0]
    stp x21, x22, [sp, #16]
    stp x23, x24, [sp, #32]
    stp x25, x26, [sp, #48]
    stp x27, x28, [sp, #64]
    stp x29, x30, [sp, #80]
    mov x9, sp
    str x9, [x0]
    mov x9, x1
    mov x10, x2
    mov x11, x3
    mov x12, x4
    mov x13, x5
    mov x0, sp
    mov x1, x10
    mov x2, x11
    mov x3, x12
    mov x4, x13
    mov x5, x9
    bl fork_copy
    ldp x19, x20, [sp, #0]
    ldp x21, x22, [sp, #16]
    ldp x23, x24, [sp, #32]
    ldp x25, x26, [sp, #48]
    ldp x27, x28, [sp, #64]
    ldp x29, x30, [sp, #80]
    add sp, sp, #96
    ret

    .global context_switch
context_switch:
    sub sp, sp, #96
    stp x19, x20, [sp, #0]
    stp x21, x22, [sp, #16]
    stp x23, x24, [sp, #32]
    stp x25, x26, [sp, #48]
    stp x27, x28, [sp, #64]
    stp x29, x30, [sp, #80]
    mov x9, sp
    str x9, [x0]
    ldr x9, [x1]
    mov sp, x9
    ldp x19, x20, [sp, #0]
    ldp x21, x22, [sp, #16]
    ldp x23, x24, [sp, #32]
    ldp x25, x26, [sp, #48]
    ldp x27, x28, [sp, #64]
    ldp x29, x30, [sp, #80]
    add sp, sp, #96
    ret
    "#
);
