//! Several slab types with their own APIs/tradeoffs.
//!
//! Some attempt is made to have the reuse/build on each other. For example:
//! * KeyedRcSlab -> RcSlab -> GenSlab
//!
//! GenSlab probably could contain a RawSlab, but doesn't for now
//!
//! Most operations are O(1), but there is risk of having to resize a vector. It's recommended in
//! a shipped game that you pre-allocate the size you need to avoid this.

///TODO: In debug mode, we could put protections for using a key with the wrong slab
///TODO: Better diagnostics/tracking

/// Scalar type for tracking element generation
///
/// u32 should be enough, even at 120fps, one allocation per frame, it would take
/// more than a year to exhaust
pub type GenerationCounterT = u32;

/// Scalar type for the count of elements of a T
///
/// Realistically we shouldn't have 4 billion of something.. and if we do, it's reasonable to expect
/// someone to write custom storage code for it
pub type SlabIndexT = u32;

mod raw_slab;
pub use raw_slab::RawSlab;
pub use raw_slab::RawSlabKey;

// GenSlab not currently in use anywhere

//mod generation;
//pub use generation::Generation;
//pub use generation::GenerationIndex;

//mod gen_slab;
// pub use gen_slab::GenSlab;
// pub use gen_slab::GenSlabKey;
