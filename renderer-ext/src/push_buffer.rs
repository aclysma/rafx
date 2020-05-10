
pub struct PushBufferResult {
    offset: usize,
    size: usize,
}

impl PushBufferResult {
    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

pub struct PushBufferSizeCalculator {
    required_size: usize,
}

impl PushBufferSizeCalculator {
    pub fn new() -> Self {
        PushBufferSizeCalculator { required_size: 0 }
    }

    pub fn push_bytes(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) {
        self.push(data, required_alignment)
    }

    pub fn push<T>(
        &mut self,
        data: &[T],
        required_alignment: usize,
    ) {
        self.required_size = ((self.required_size + required_alignment - 1) / required_alignment)
            * required_alignment;
        self.required_size += (data.len() * std::mem::size_of::<T>());
    }

    pub fn required_size(&self) -> usize {
        self.required_size
    }
}

pub struct PushBuffer {
    data: Vec<u8>,
}

impl PushBuffer {
    pub fn new(size_hint: usize) -> Self {
        PushBuffer {
            data: Vec::with_capacity(size_hint),
        }
    }

    pub fn push_bytes(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) -> PushBufferResult {
        // Figure out where in the buffer to write
        let span_begin =
            ((self.data.len() + required_alignment - 1) / required_alignment) * required_alignment;
        let span_end = span_begin + data.len();

        // Resize the buffer and copy the data
        self.data.resize(span_end, 0);
        self.data[span_begin..span_end].copy_from_slice(data);

        // Return the offset
        PushBufferResult {
            offset: span_begin,
            size: data.len(),
        }
    }

    pub fn push<T>(
        &mut self,
        data: &[T],
        required_alignment: usize,
    ) -> PushBufferResult {
        let ptr: *const u8 = data.as_ptr() as *const u8;
        let slice: &[u8] =
            unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<T>() * data.len()) };

        self.push_bytes(slice, required_alignment)
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}