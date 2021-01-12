use std::hash::Hasher;

// This is an f32 that supports Hash and Eq. Generally this is dangerous, but here we're
// not doing any sort of fp-arithmetic and not expecting NaN. We should be deterministically
// parsing a string and creating a float from it.
#[derive(Debug, Copy, Clone, Default)]
pub struct DecimalF32(pub f32);

impl Into<f32> for DecimalF32 {
    fn into(self) -> f32 {
        self.0
    }
}

impl Into<i32> for DecimalF32 {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

impl Into<u32> for DecimalF32 {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl PartialEq for DecimalF32 {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0 == other.0
    }
}

impl Eq for DecimalF32 {}

impl std::hash::Hash for DecimalF32 {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        let bits: u32 = self.0.to_bits();
        bits.hash(state);
    }
}

// This is an f64 that supports Hash and Eq. Generally this is dangerous, but here we're
// not doing any sort of fp-arithmetic and not expecting NaN. We should be deterministically
// parsing a string and creating a float from it.
#[derive(Debug, Copy, Clone, Default)]
pub struct DecimalF64(pub f64);

impl Into<f64> for DecimalF64 {
    fn into(self) -> f64 {
        self.0
    }
}

impl Into<f32> for DecimalF64 {
    fn into(self) -> f32 {
        self.0 as f32
    }
}

impl Into<i32> for DecimalF64 {
    fn into(self) -> i32 {
        self.0 as i32
    }
}

impl Into<u32> for DecimalF64 {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl PartialEq for DecimalF64 {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0 == other.0
    }
}

impl Eq for DecimalF64 {}

impl std::hash::Hash for DecimalF64 {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        let bits: u64 = self.0.to_bits();
        bits.hash(state);
    }
}
