use glam::Vec3;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::{mem, slice};

#[derive(Clone, Debug)]
pub struct PolygonSoup {
    pub vertex_positions: Vec<Vec3>,
    pub index: PolygonSoupIndex,
}

#[derive(Clone, Debug)]
pub enum PolygonSoupIndex {
    None,
    Indexed16(Vec<u16>),
    Indexed32(Vec<u32>),
}

impl Hash for PolygonSoup {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        fn as_raw_bytes<T>(value: &T) -> &[u8] {
            let p: *const T = value;
            let p: *const u8 = p as *const u8;
            unsafe { slice::from_raw_parts(p, mem::size_of::<T>()) }
        }

        for vertex in self.vertex_positions.iter() {
            let vertex_data = as_raw_bytes(vertex);
            vertex_data.hash(state);
        }

        match &self.index {
            PolygonSoupIndex::None => {}
            PolygonSoupIndex::Indexed16(indices) => indices.hash(state),
            PolygonSoupIndex::Indexed32(indices) => indices.hash(state),
        }
    }
}

impl PolygonSoup {
    pub fn calculate_hash(&self) -> u64 {
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        hash.finish()
    }
}
