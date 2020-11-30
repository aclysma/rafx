use crate::vk_description as dsc;
use crate::vk_description::Format;
use fnv::FnvHashMap;
use std::collections::BTreeMap;
use std::sync::Arc;

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VertexDataLayoutHash(u64);

impl VertexDataLayoutHash {
    fn new(
        vertex_size: usize,
        members: &FnvHashMap<String, VertexDataMemberMeta>,
    ) -> VertexDataLayoutHash {
        // Put everything in the BTree so that we get a deterministic sort
        let mut sorted = BTreeMap::<&String, &VertexDataMemberMeta>::default();
        for (key, value) in members {
            sorted.insert(key, value);
        }

        // Hash the vertex size/btree
        use fnv::FnvHasher;
        use std::hash::Hash;
        use std::hash::Hasher;
        let mut hasher = FnvHasher::default();
        vertex_size.hash(&mut hasher);
        sorted.hash(&mut hasher);
        VertexDataLayoutHash(hasher.finish())
    }
}

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VertexDataSetLayoutHash(u64);

impl VertexDataSetLayoutHash {
    fn new(layouts: &[VertexDataLayout]) -> VertexDataSetLayoutHash {
        // Hash the hashes
        use fnv::FnvHasher;
        use std::hash::Hash;
        use std::hash::Hasher;
        let mut hasher = FnvHasher::default();
        for layout in layouts {
            layout.hash().0.hash(&mut hasher);
        }
        VertexDataSetLayoutHash(hasher.finish())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VertexCopyError {
    VertexCountDoesNotMatch,
    MemberFormatDoesNotMatch,
    SizeOfSliceTypeDoesNotMatchLayout,
    CantReinitializeFrom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VertexMember {
    pub semantic: String,
    pub format: dsc::Format,
    pub offset: usize,
}

// Copy/'static constraints ensure we only pass simple value types. Anything else is almost certainly a mistake
impl VertexMember {
    pub fn new<VertexT: Copy + 'static, MemberT: Copy + 'static>(
        vertex: &VertexT,
        member: &MemberT,
        semantic: String,
        format: Format,
    ) -> VertexMember {
        // Get ptrs as usize to do some math
        let vertex_addr = vertex as *const _ as usize;
        let member_addr = member as *const _ as usize;

        // Verify that MemberT is actually a member of the vertex. Check that the member pointer
        // is at or after the vertex pointer. Also check that the member pointer + member size is
        // less than the vertex pointer + vertex size
        let rust_member_size = std::mem::size_of::<MemberT>();
        assert!(member_addr >= vertex_addr);
        assert!((member_addr + rust_member_size) <= (vertex_addr + std::mem::size_of::<VertexT>()));

        // Check that the provided format is size-compatible with the rust type
        let format_size = dsc::size_of_vertex_format(format).expect(
            "The provided format {:?} has an unknown size. Is it valid to use as vertex data?",
        );
        assert_eq!(
            rust_member_size,
            format_size,
            "The provided format {:?} is {} bytes but the rust type {} is {} bytes",
            format,
            format_size,
            std::any::type_name::<MemberT>(),
            rust_member_size
        );

        let offset = member_addr - vertex_addr;
        VertexMember {
            semantic,
            format,
            offset,
        }
    }
}

pub struct VertexMemberAccumulator<'a, VertexT> {
    members: Vec<VertexMember>,
    vertex: &'a VertexT,
}

// Copy/'static constraints ensure we only pass simple value types. Anything else is almost certainly a mistake
impl<'a, VertexT: Copy + 'static> VertexMemberAccumulator<'a, VertexT> {
    pub fn add_member<MemberT: Copy + 'static, StrT: Into<String>>(
        &mut self,
        member: &MemberT,
        semantic: StrT,
        format: Format,
    ) {
        self.members.push(VertexMember::new(
            self.vertex,
            member,
            semantic.into(),
            format,
        ));
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct VertexDataMemberMeta {
    pub format: dsc::Format,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, PartialEq)]
pub struct VertexDataLayoutInner {
    //TODO: Change strings to hashes
    //TODO: Not clear if hashmap is better than linear or binary search on few elements
    members: FnvHashMap<String, VertexDataMemberMeta>,
    vertex_size: usize,
    hash: VertexDataLayoutHash,
}

#[derive(Debug, PartialEq, Clone)]
pub struct VertexDataLayout {
    //TODO: Change strings to hashes
    //TODO: Not clear if hashmap is better than linear or binary search on few elements
    inner: Arc<VertexDataLayoutInner>,
}

impl VertexDataLayout {
    pub fn new(
        vertex_size: usize,
        members: &[VertexMember],
    ) -> Self {
        let mut map = Default::default();
        for member in members {
            Self::add_member_to_map(vertex_size, &mut map, member);
        }

        let hash = VertexDataLayoutHash::new(vertex_size, &map);

        let inner = VertexDataLayoutInner {
            members: map,
            vertex_size,
            hash,
        };

        VertexDataLayout {
            inner: Arc::new(inner),
        }
    }

    /// Example usage to define a vertex data layout and convert it into a VertexDataSetLayout
    /// (`VertexDataLayout::build_vertex_layout` creates a VertexDataLayout and the `.into_set()`
    /// call makes it a single-buffer set
    ///
    /// ```
    /// use rafx_resources::VertexDataLayout;
    /// use rafx_resources::vk_description::Format;
    ///
    /// #[derive(Default, Copy, Clone)]
    /// #[repr(C)]
    /// pub struct ExampleVertex {
    ///     pub position: [f32; 3],
    ///     pub normal: [f32; 3],
    ///     pub tex_coord: [f32; 2],
    /// }
    ///
    /// VertexDataLayout::build_vertex_layout(&ExampleVertex::default(), |builder, vertex| {
    ///     builder.add_member(&vertex.position, "POSITION", Format::R32G32B32_SFLOAT);
    ///     builder.add_member(&vertex.normal, "NORMAL", Format::R32G32B32_SFLOAT);
    ///     builder.add_member(&vertex.tex_coord, "TEXCOORD", Format::R32G32_SFLOAT);
    /// }).into_set();
    /// ```
    pub fn build_vertex_layout<
        VertexT,
        F: FnOnce(&mut VertexMemberAccumulator<VertexT>, &VertexT),
    >(
        vertex: &VertexT,
        f: F,
    ) -> VertexDataLayout {
        let mut accumulator = VertexMemberAccumulator {
            members: Default::default(),
            vertex,
        };

        (f)(&mut accumulator, vertex);
        VertexDataLayout::new(std::mem::size_of::<VertexT>(), &accumulator.members)
    }

    fn add_member_to_map(
        vertex_size: usize,
        map: &mut FnvHashMap<String, VertexDataMemberMeta>,
        member: &VertexMember,
    ) {
        let size = dsc::size_of_vertex_format(member.format).unwrap();
        assert!(member.offset + size <= vertex_size);
        let old = map.insert(
            member.semantic.clone(),
            VertexDataMemberMeta {
                format: member.format,
                offset: member.offset,
                size,
            },
        );
        assert!(old.is_none());
    }

    pub fn member(
        &self,
        name: &str,
    ) -> Option<&VertexDataMemberMeta> {
        self.inner.members.get(name)
    }

    pub fn members(&self) -> &FnvHashMap<String, VertexDataMemberMeta> {
        &self.inner.members
    }

    pub fn hash(&self) -> VertexDataLayoutHash {
        self.inner.hash
    }

    pub fn vertex_size(&self) -> usize {
        self.inner.vertex_size
    }

    pub fn into_set(self) -> VertexDataSetLayout {
        VertexDataSetLayout::new(vec![self])
    }

    pub fn intersects_with(
        &self,
        other: &Self,
    ) -> bool {
        for member_name in self.inner.members.keys() {
            if other.inner.members.contains_key(member_name) {
                return true;
            }
        }

        false
    }

    pub fn is_subset_of(
        &self,
        other: &Self,
    ) -> bool {
        for member_name in self.inner.members.keys() {
            if !other.inner.members.contains_key(member_name) {
                return false;
            }
        }

        true
    }

    pub fn is_subset_of_multi(
        subsets: &[Self],
        others: &[Self],
    ) -> bool {
        for subset in subsets {
            for member_name in subset.inner.members.keys() {
                let mut found_in_other = false;
                for other in others {
                    if other.inner.members.contains_key(member_name) {
                        found_in_other = true;
                        break;
                    }
                }

                if !found_in_other {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VertexDataSetMemberMeta {
    pub format: dsc::Format,
    pub offset: usize,
    pub size: usize,
    pub binding: usize,
}

#[derive(Debug, PartialEq)]
pub struct VertexDataSetLayoutInner {
    //TODO: Change strings to hashes
    //TODO: Not clear if hashmap is better than linear or binary search on few elements
    members: Arc<FnvHashMap<String, VertexDataSetMemberMeta>>,
    layouts: Vec<VertexDataLayout>,
    hash: VertexDataSetLayoutHash,
}

#[derive(Debug, PartialEq)]
pub struct VertexDataSetLayout {
    inner: Arc<VertexDataSetLayoutInner>,
}

impl VertexDataSetLayout {
    pub fn new(layouts: Vec<VertexDataLayout>) -> Self {
        let mut members = FnvHashMap::default();
        for (binding, layout) in layouts.iter().enumerate() {
            for (member_name, meta) in &layout.inner.members {
                let old = members.insert(
                    member_name.clone(),
                    VertexDataSetMemberMeta {
                        format: meta.format,
                        size: meta.size,
                        offset: meta.offset,
                        binding,
                    },
                );
                assert!(old.is_none());
            }
        }

        let hash = VertexDataSetLayoutHash::new(&layouts);

        let inner = VertexDataSetLayoutInner {
            members: Arc::new(members),
            layouts,
            hash,
        };

        VertexDataSetLayout {
            inner: Arc::new(inner),
        }
    }

    pub fn binding(
        &self,
        binding: usize,
    ) -> Option<&VertexDataLayout> {
        self.inner.layouts.get(binding)
    }

    pub fn bindings(&self) -> &Vec<VertexDataLayout> {
        &self.inner.layouts
    }

    pub fn member(
        &self,
        name: &str,
    ) -> Option<&VertexDataSetMemberMeta> {
        self.inner.members.get(name)
    }

    pub fn members(&self) -> &FnvHashMap<String, VertexDataSetMemberMeta> {
        &self.inner.members
    }

    pub fn hash(&self) -> VertexDataSetLayoutHash {
        self.inner.hash
    }
}

#[derive(Clone)]
pub struct VertexData {
    layout: VertexDataLayout,
    // Align to 16 bytes to try to stay clear of alignment issues
    data: Vec<u128>,
    vertex_count: usize,
}

impl VertexData {
    pub fn new_memzero(
        layout: VertexDataLayout,
        vertex_count: usize,
    ) -> Self {
        let total_size = layout.vertex_size() * vertex_count;

        // Allocate 16-byte aligned blob of memory that is large enough to contain the data
        let data = vec![0_u128; (total_size + 15) / 16];

        VertexData {
            layout,
            data,
            vertex_count,
        }
    }

    pub fn new_from_slice<T: Copy>(
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Self {
        let mut data = Self::new_memzero(src_layout.clone(), src_data.len());
        data.copy_from_slice(src_layout, src_data).unwrap();
        data
    }

    pub fn reinitialize_from(
        &mut self,
        other: &Self,
    ) -> Result<(), VertexCopyError> {
        if !self.layout.is_subset_of(&other.layout) {
            return Err(VertexCopyError::CantReinitializeFrom);
        }

        self.copy_from(other)
    }

    pub fn reinitialize_from_slice<T: Copy>(
        &mut self,
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Result<(), VertexCopyError> {
        if !self.layout.is_subset_of(&src_layout) {
            return Err(VertexCopyError::CantReinitializeFrom);
        }

        self.copy_from_slice(src_layout, src_data)
    }

    pub fn copy_from(
        &mut self,
        other: &Self,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(other, self)
    }

    pub fn copy_to(
        &self,
        other: &mut Self,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(self, other)
    }

    pub fn copy_between_vertex_data(
        src: &VertexData,
        dst: &mut VertexData,
    ) -> Result<(), VertexCopyError> {
        if src.vertex_count != dst.vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        unsafe {
            let src_ptr = src.ptr();
            let dst_ptr = dst.ptr_mut();
            Self::copy_between_layouts(&src.layout, src_ptr, &dst.layout, dst_ptr, dst.vertex_count)
        }
    }

    pub fn copy_from_slice<T: Copy>(
        &mut self,
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Result<(), VertexCopyError> {
        if src_data.len() != self.vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        if std::mem::size_of::<T>() != src_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        unsafe {
            let dst_data = self.ptr_mut();
            Self::copy_between_layouts(
                src_layout,
                src_data.as_ptr() as *const u8,
                &self.layout,
                dst_data,
                src_data.len(),
            )
        }
    }

    pub fn copy_to_slice<T: Copy>(
        &self,
        dst_layout: &VertexDataLayout,
        dst_data: &mut [T],
    ) -> Result<(), VertexCopyError> {
        if dst_data.len() != self.vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        if std::mem::size_of::<T>() != dst_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        unsafe {
            let src_data = self.ptr();
            Self::copy_between_layouts(
                &self.layout,
                src_data,
                dst_layout,
                dst_data.as_mut_ptr() as *mut u8,
                dst_data.len(),
            )
        }
    }

    pub fn copy_between_slices<T: Copy, U: Copy>(
        src_layout: &VertexDataLayout,
        src_data: &[T],
        dst_layout: &VertexDataLayout,
        dst_data: &mut [U],
    ) -> Result<(), VertexCopyError> {
        if src_data.len() != dst_data.len() {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        if std::mem::size_of::<T>() != src_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        if std::mem::size_of::<U>() != dst_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        unsafe {
            Self::copy_between_layouts(
                src_layout,
                src_data.as_ptr() as *const u8,
                dst_layout,
                dst_data.as_mut_ptr() as *mut u8,
                dst_data.len(),
            )
        }
    }

    pub fn can_copy_between_layouts(
        src_layout: &VertexDataLayout,
        dst_layout: &VertexDataLayout,
    ) -> Result<(), VertexCopyError> {
        // Verify the copies will succeed before starting
        for (member_name, src_member) in src_layout.members() {
            if let Some(dst_member) = dst_layout.members().get(member_name) {
                if src_member.format != dst_member.format {
                    return Err(VertexCopyError::MemberFormatDoesNotMatch);
                }

                // Should always pass because we check that the formats are identical
                assert_eq!(src_member.size, dst_member.size);
            }
        }

        Ok(())
    }

    pub unsafe fn copy_between_layouts(
        src_layout: &VertexDataLayout,
        src_data: *const u8,
        dst_layout: &VertexDataLayout,
        dst_data: *mut u8,
        vertex_count: usize,
    ) -> Result<(), VertexCopyError> {
        if src_layout == dst_layout {
            std::ptr::copy_nonoverlapping(
                src_data,
                dst_data,
                vertex_count * src_layout.vertex_size(),
            );
            return Ok(());
        }

        if !src_layout.intersects_with(dst_layout) {
            return Ok(());
        }

        // Verify the copies will succeed before starting
        Self::can_copy_between_layouts(src_layout, dst_layout)?;

        //TODO: Would it be faster to do per-vertex instead of per-member?
        for (member_name, src_member) in src_layout.members() {
            if let Some(dst_member) = dst_layout.members().get(member_name) {
                for i in 0..vertex_count {
                    let src_ptr = src_data.add((src_layout.vertex_size() * i) + src_member.offset);
                    let dst_ptr = dst_data.add((dst_layout.vertex_size() * i) + dst_member.offset);

                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_member.size);
                }
            }
        }

        Ok(())
    }

    pub unsafe fn ptr(&self) -> *const u8 {
        self.data.as_ptr() as *const u8
    }

    pub unsafe fn ptr_mut(&mut self) -> *mut u8 {
        self.data.as_mut_ptr() as *mut u8
    }
}

pub struct VertexDataSet {
    layouts: Vec<VertexDataLayout>,
    data: Vec<VertexData>,
    vertex_count: usize,
}

impl VertexDataSet {
    pub fn new(data: Vec<VertexData>) -> Result<Self, &'static str> {
        if data.is_empty() {
            Ok(VertexDataSet {
                data: Default::default(),
                vertex_count: 0,
                layouts: Default::default(),
            })
        } else {
            let vertex_count = data[0].vertex_count;
            for d in &data {
                if vertex_count != d.vertex_count {
                    return Err("vertex data does not have same number of vertices");
                }
            }

            let layouts = data.iter().map(|x| x.layout.clone()).collect();

            Ok(VertexDataSet {
                data,
                vertex_count,
                layouts,
            })
        }
    }

    pub fn new_memzero(
        layouts: &[VertexDataLayout],
        vertex_count: usize,
    ) -> Self {
        let data = layouts
            .iter()
            .map(|layout| VertexData::new_memzero(layout.clone(), vertex_count))
            .collect();

        VertexDataSet {
            data,
            vertex_count,
            layouts: layouts.to_vec(),
        }
    }

    pub fn new_from_slice<T: Copy>(
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Self {
        let mut data = vec![VertexData::new_memzero(src_layout.clone(), src_data.len())];
        data[0].copy_from_slice(src_layout, src_data).unwrap();

        VertexDataSet {
            data,
            vertex_count: src_data.len(),
            layouts: vec![src_layout.clone()],
        }
    }

    pub fn reinitialize_from(
        &mut self,
        other: &Self,
    ) -> Result<(), VertexCopyError> {
        if !VertexDataLayout::is_subset_of_multi(&self.layouts, &other.layouts) {
            return Err(VertexCopyError::CantReinitializeFrom);
        }

        self.copy_from(other)
    }

    pub fn reinitialize_from_slice<T: Copy>(
        &mut self,
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Result<(), VertexCopyError> {
        for layout in &self.layouts {
            if !layout.is_subset_of(src_layout) {
                return Err(VertexCopyError::CantReinitializeFrom);
            }
        }

        self.copy_from_slice(src_layout, src_data)
    }

    pub fn copy_from(
        &mut self,
        other: &Self,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(
            other.vertex_count,
            &other.data,
            self.vertex_count,
            &mut self.data,
        )
    }

    pub fn copy_to(
        &self,
        other: &mut Self,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(
            self.vertex_count,
            &self.data,
            other.vertex_count,
            &mut other.data,
        )
    }

    pub fn copy_from_single(
        &mut self,
        other: &VertexData,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(
            other.vertex_count,
            std::slice::from_ref(other),
            self.vertex_count,
            &mut self.data,
        )
    }

    pub fn copy_to_single(
        &self,
        other: &mut VertexData,
    ) -> Result<(), VertexCopyError> {
        Self::copy_between_vertex_data(
            self.vertex_count,
            &self.data,
            other.vertex_count,
            std::slice::from_mut(other),
        )
    }

    pub fn copy_between_vertex_data(
        src_vertex_count: usize,
        src_data: &[VertexData],
        dst_vertex_count: usize,
        dst_data: &mut [VertexData],
    ) -> Result<(), VertexCopyError> {
        if src_vertex_count != dst_vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        for src in src_data {
            for dst in &mut *dst_data {
                VertexData::can_copy_between_layouts(&src.layout, &dst.layout)?;
            }
        }

        for src in src_data {
            for dst in &mut *dst_data {
                dst.copy_from(src)?;
            }
        }

        Ok(())
    }

    pub fn copy_from_slice<T: Copy>(
        &mut self,
        src_layout: &VertexDataLayout,
        src_data: &[T],
    ) -> Result<(), VertexCopyError> {
        if src_data.len() != self.vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        if std::mem::size_of::<T>() != src_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        for layout in &self.layouts {
            VertexData::can_copy_between_layouts(src_layout, layout)?;
        }

        for data in &mut self.data {
            data.copy_from_slice(src_layout, src_data)?;
        }

        Ok(())
    }

    pub fn copy_to_slice<T: Copy>(
        &self,
        dst_layout: &VertexDataLayout,
        dst_data: &mut [T],
    ) -> Result<(), VertexCopyError> {
        if dst_data.len() != self.vertex_count {
            return Err(VertexCopyError::VertexCountDoesNotMatch);
        }

        if std::mem::size_of::<T>() != dst_layout.vertex_size() {
            return Err(VertexCopyError::SizeOfSliceTypeDoesNotMatchLayout);
        }

        for layout in &self.layouts {
            VertexData::can_copy_between_layouts(layout, dst_layout)?;
        }

        for data in &self.data {
            data.copy_to_slice(dst_layout, dst_data)?;
        }

        Ok(())
    }

    pub unsafe fn data(&self) -> &[VertexData] {
        &self.data
    }

    pub unsafe fn data_mut(&mut self) -> &mut [VertexData] {
        &mut self.data
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::vk_description::Format;

    #[derive(Default, Clone, Copy, Debug)]
    #[repr(C)]
    pub struct MediumVertex {
        pub position: [f32; 3],
        pub normal: [f32; 3],
        pub color: [f32; 4],
        pub tangent: [f32; 3],
        pub tex_coord: [f32; 2],
    }

    impl MediumVertex {
        fn get_layout() -> VertexDataLayout {
            VertexDataLayout::build_vertex_layout(&Self::default(), |builder, vertex| {
                builder.add_member(&vertex.position, "POSITION", Format::R32G32B32_SFLOAT);
                builder.add_member(&vertex.normal, "NORMAL", Format::R32G32B32_SFLOAT);
                builder.add_member(&vertex.color, "COLOR", Format::R32G32B32A32_SFLOAT);
                builder.add_member(&vertex.tangent, "TANGENT", Format::R32G32B32_SFLOAT);
                builder.add_member(&vertex.tex_coord, "TEXCOORD", Format::R32G32_SFLOAT);
            })
        }

        fn create_empty_data() -> Vec<MediumVertex> {
            (0..100).map(|_| MediumVertex::default()).collect()
        }

        fn create_test_data() -> Vec<MediumVertex> {
            (0..100)
                .map(|x| {
                    let x = x as f32;
                    MediumVertex {
                        position: [1.0, x, x],
                        normal: [2.0, x, x],
                        color: [3.0, x, x, x],
                        tangent: [4.0, x, x],
                        tex_coord: [5.0, x],
                    }
                })
                .collect()
        }
    }

    #[derive(Default, Clone, Copy, Debug)]
    #[repr(C)]
    pub struct SmallVertex {
        pub normal: [f32; 3],
        pub position: [f32; 3],
    }

    impl SmallVertex {
        fn get_layout() -> VertexDataLayout {
            VertexDataLayout::build_vertex_layout(&Self::default(), |builder, vertex| {
                builder.add_member(&vertex.position, "POSITION", Format::R32G32B32_SFLOAT);
                builder.add_member(&vertex.normal, "NORMAL", Format::R32G32B32_SFLOAT);
            })
        }

        fn create_empty_data() -> Vec<SmallVertex> {
            (0..100).map(|_| SmallVertex::default()).collect()
        }

        fn create_test_data() -> Vec<SmallVertex> {
            (0..100)
                .map(|x| {
                    let x = x as f32;
                    SmallVertex {
                        position: [1.0, x, x],
                        normal: [2.0, x, x],
                    }
                })
                .collect()
        }
    }

    #[derive(Default, Clone, Copy, Debug)]
    #[repr(C)]
    pub struct TinyVertex {
        pub color: [f32; 4],
    }

    impl TinyVertex {
        fn get_layout() -> VertexDataLayout {
            VertexDataLayout::build_vertex_layout(&Self::default(), |builder, vertex| {
                builder.add_member(&vertex.color, "COLOR", Format::R32G32B32A32_SFLOAT);
            })
        }
    }

    #[test]
    fn test_to_smaller() {
        let from_layout = MediumVertex::get_layout();
        let from_data = MediumVertex::create_test_data();

        let to_layout = SmallVertex::get_layout();
        let mut to_data = SmallVertex::create_empty_data();

        let data = VertexData::new_from_slice(&from_layout, &from_data);
        data.copy_to_slice(&to_layout, &mut to_data).unwrap();

        assert!((from_data[4].position[1] - 4.0).abs() < 0.1);
        assert!((to_data[4].position[1] - 4.0).abs() < 0.1);

        assert!((from_data[4].position[0] - 1.0).abs() < 0.1);
        assert!((to_data[4].position[0] - 1.0).abs() < 0.1);

        assert!((from_data[4].normal[0] - 2.0).abs() < 0.1);
        assert!((to_data[4].normal[0] - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_to_larger() {
        let from_layout = SmallVertex::get_layout();
        let from_data = SmallVertex::create_test_data();

        let to_layout = MediumVertex::get_layout();
        let mut to_data = MediumVertex::create_empty_data();

        let data = VertexData::new_from_slice(&from_layout, &from_data);
        data.copy_to_slice(&to_layout, &mut to_data).unwrap();

        assert!((from_data[4].position[1] - 4.0).abs() < 0.1);
        assert!((to_data[4].position[1] - 4.0).abs() < 0.1);

        assert!((from_data[4].position[0] - 1.0).abs() < 0.1);
        assert!((to_data[4].position[0] - 1.0).abs() < 0.1);

        assert!((from_data[4].normal[0] - 2.0).abs() < 0.1);
        assert!((to_data[4].normal[0] - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_copy_from() {
        let from_layout = MediumVertex::get_layout();
        let from_data = MediumVertex::create_test_data();
        let from = VertexData::new_from_slice(&from_layout, &from_data);

        let to_layout = SmallVertex::get_layout();
        let to_data = SmallVertex::create_empty_data();
        let mut to = VertexData::new_from_slice(&to_layout, &to_data);

        to.copy_from(&from).unwrap();

        let mut to_data = SmallVertex::create_empty_data();
        to.copy_to_slice(&to_layout, &mut to_data).unwrap();

        assert!((from_data[4].position[1] - 4.0).abs() < 0.1);
        assert!((to_data[4].position[1] - 4.0).abs() < 0.1);

        assert!((from_data[4].position[0] - 1.0).abs() < 0.1);
        assert!((to_data[4].position[0] - 1.0).abs() < 0.1);

        assert!((from_data[4].normal[0] - 2.0).abs() < 0.1);
        assert!((to_data[4].normal[0] - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_into_data_set() {
        let from_layout = MediumVertex::get_layout();
        let from_data = MediumVertex::create_test_data();
        let from = VertexData::new_from_slice(&from_layout, &from_data);

        let mut data_set = VertexDataSet::new(vec![
            VertexData::new_memzero(SmallVertex::get_layout(), 100),
            VertexData::new_memzero(TinyVertex::get_layout(), 100),
        ])
        .unwrap();

        data_set.copy_from_single(&from).unwrap();

        let mut to_data = SmallVertex::create_empty_data();
        data_set
            .copy_to_slice(&SmallVertex::get_layout(), &mut to_data)
            .unwrap();

        assert!((from_data[4].position[1] - 4.0).abs() < 0.1);
        assert!((to_data[4].position[1] - 4.0).abs() < 0.1);

        assert!((from_data[4].position[0] - 1.0).abs() < 0.1);
        assert!((to_data[4].position[0] - 1.0).abs() < 0.1);

        assert!((from_data[4].normal[0] - 2.0).abs() < 0.1);
        assert!((to_data[4].normal[0] - 2.0).abs() < 0.1);
    }
}
