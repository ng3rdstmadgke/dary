use std::slice;
use std::mem;

pub fn to_bytes<U>(slice: &[U]) -> &[u8] {
    unsafe {
        slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            mem::size_of::<U>() * slice.len()
        )
    }
}
