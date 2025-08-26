use serde_json::json;

// Simple bump allocator helpers for host I/O
#[no_mangle]
pub extern "C" fn alloc(size: u32) -> *mut u8 {
    // Allocate a Vec<u8> with the requested capacity and leak it to get a stable pointer.
    let mut buf = Vec::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: u32) {
    if !ptr.is_null() && size > 0 {
        unsafe { let _ = Vec::from_raw_parts(ptr, size as usize, size as usize); }
    }
}

// Execute expects JSON utf-8 input at (in_ptr, in_len) and returns a packed u64 with (out_ptr<<32 | out_len)
#[no_mangle]
pub extern "C" fn execute(in_ptr: *const u8, in_len: u32) -> u64 {
    if in_ptr.is_null() || in_len == 0 { return 0; }
    let input_str = unsafe {
        let slice = core::slice::from_raw_parts(in_ptr, in_len as usize);
        match core::str::from_utf8(slice) { Ok(s) => s, Err(_) => "" }
    };

    // Produce a simple echo JSON: {"echo": <input_string>}
    let out = json!({"echo": input_str}).to_string();
    let bytes = out.into_bytes();
    let len = bytes.len() as u32;
    let mut v = bytes;
    let out_ptr = v.as_mut_ptr();
    core::mem::forget(v); // leak, host must call dealloc(out_ptr, len)

    ((out_ptr as u64) << 32) | (len as u64)
}
