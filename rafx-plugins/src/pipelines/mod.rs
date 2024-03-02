// The mesh feature requires legion
#[cfg(all(feature = "basic-pipeline", feature = "legion"))]
pub mod basic;

#[cfg(all(not(feature = "basic-pipeline"), feature = "legion"))]
pub mod modern;
