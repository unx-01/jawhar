use core::ffi::c_void;

#[no_mangle]
pub unsafe extern "C" fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void {
    let d = d as *mut u8;
    let s = s as *const u8;
    for i in 0..n {
        *d.add(i) = *s.add(i);
    }
    d as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn memset(d: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let d = d as *mut u8;
    for i in 0..n {
        *d.add(i) = c as u8;
    }
    d as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void {
    let d = d as *mut u8;
    let s = s as *const u8;
    if (d as usize) <= (s as usize) {
        for i in 0..n {
            *d.add(i) = *s.add(i);
        }
    } else {
        for i in (0..n).rev() {
            *d.add(i) = *s.add(i);
        }
    }
    d as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    let a = a as *const u8;
    let b = b as *const u8;
    for i in 0..n {
        let x = *a.add(i) as i32;
        let y = *b.add(i) as i32;
        if x != y {
            return x - y;
        }
    }
    0
}
