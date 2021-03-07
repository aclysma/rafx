pub fn round_size_up_to_alignment_u32(
    size: u32,
    required_alignment: u32,
) -> u32 {
    assert!(required_alignment > 0);
    ((size + required_alignment - 1) / required_alignment) * required_alignment
}

pub fn round_size_up_to_alignment_u64(
    size: u64,
    required_alignment: u64,
) -> u64 {
    assert!(required_alignment > 0);
    ((size + required_alignment - 1) / required_alignment) * required_alignment
}

pub fn any_as_bytes<T: Copy>(data: &T) -> &[u8] {
    let ptr: *const T = data;
    let ptr = ptr as *const u8;
    let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<T>()) };

    slice
}

pub fn slice_size_in_bytes<T>(slice: &[T]) -> usize {
    let range = slice.as_ptr_range();
    (range.end as *const u8 as usize) - (range.start as *const u8 as usize)
}

pub unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}

pub unsafe fn force_to_static_lifetime_mut<T>(value: &mut T) -> &'static mut T {
    std::mem::transmute(value)
}
