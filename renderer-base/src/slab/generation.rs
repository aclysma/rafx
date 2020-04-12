use super::GenerationCounterT;

/// Represents a particular instance of a Generation. For example, if a Generation is set,
/// cleared, then set again, the second instance will have a different generation index than the first
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct GenerationIndex(GenerationCounterT);

/// Wraps a T, requiring a generation index to access it. Used for scenarios where you have a pool of
/// Ts that may change, and you want to index into a specific instance, but with detection for if
/// the index is stale.
///
/// This data structure is assert/panic-happy because mistakes in using it can imply subtle bugs in
/// downstream code.
pub struct Generation<T> {
    /// A counter that increments when free() is called
    generation_index: GenerationIndex,

    /// Underlying T
    value: Option<T>,
}

impl<T> Default for Generation<T> {
    fn default() -> Self {
        Generation {
            generation_index: GenerationIndex(0),
            value: None
        }
    }
}

impl<T> Generation<T> {
    /// Create a cleared Generation<T>
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns true if the element is not None, and matches the given generation
    pub fn exists(
        &self,
        generation: GenerationIndex,
    ) -> bool {
        self.value.is_some() && self.generation_index == generation
    }

    /// Get the value, but only if the given generation index isn't stale
    pub fn get(
        &self,
        generation: GenerationIndex,
    ) -> Option<&T> {
        //println!("get self: {} param: {}", self.generation_index.0, generation.0);

        let value = self.value.as_ref()?;
        if self.generation_index == generation {
            Some(value)
        } else {
            None
        }
    }

    /// Get the value, but only if the given generation index isn't stale
    pub fn get_mut(
        &mut self,
        generation: GenerationIndex,
    ) -> Option<&mut T> {
        //println!("get self: {} param: {}", self.generation_index.0, generation.0);

        let value = self.value.as_mut()?;
        if self.generation_index == generation {
            Some(value)
        } else {
            None
        }
    }

    /// Set the value. Fatal if a value already exists. It's called allocate to imply that you
    /// must call free before calling allocate again. (Partly to detect errors in usage, and partly
    /// because free increments generation_index
    pub fn allocate(
        &mut self,
        value: T,
    ) -> GenerationIndex {
        assert!(
            self.value.is_none(),
            "Can only allocate a generation if it's not already allocated"
        );
        self.value = Some(value);

        //println!("allocate generation {}", self.generation_index.0);
        self.generation_index
    }

    /// Clear the value. Fatal if the generation index is stale.
    pub fn free(
        &mut self,
        generation_index: GenerationIndex,
    ) {
        assert!(
            self.value.is_some(),
            "Can only free a generation if it's not already freed"
        );
        assert!(
            self.generation_index == generation_index,
            "Can not free a generation with incorrect generation_index"
        );
        self.value = None;
        self.generation_index.0 += 1;
        //println!("free generation {}", self.generation_index.0);
    }

    /// Returns true if no value exists
    pub fn is_none(&self) -> bool {
        self.value.is_none()
    }

    /// Get a ref to the inner value, but without checking the generation
    pub fn get_unchecked(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Get a mut ref to the inner value, but without checking the generation
    pub fn get_unchecked_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }

    /// Get the current generation index.
    pub fn generation_index(&self) -> GenerationIndex {
        assert!(!self.is_none());
        self.generation_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_get() {
        // Generations starts unallocated
        let mut value = Generation::new();
        assert!(value.get(GenerationIndex(0)).is_none());

        // Once it's allocated, the first gen index will work to access it
        let generation_index0 = value.allocate(0);
        assert!(value.get(generation_index0).is_some());

        // Now that it's free, the generation won't work
        value.free(generation_index0);
        assert!(value.get(generation_index0).is_none());

        // Allocate again, the new index works and the old one doesn't
        let generation_index1 = value.allocate(0);
        assert!(value.get(generation_index0).is_none());
        assert!(value.get(generation_index1).is_some());
    }

    #[test]
    fn test_generation_get_mut() {
        // Generations starts unallocated
        let mut value = Generation::new();
        assert!(value.get_mut(GenerationIndex(0)).is_none());

        // Once it's allocated, the first gen index will work to access it
        let generation_index0 = value.allocate(0);
        assert!(value.get_mut(generation_index0).is_some());

        // Now that it's free, the generation won't work
        value.free(generation_index0);
        assert!(value.get_mut(generation_index0).is_none());

        // Allocate again, the new index works and the old one doesn't
        let generation_index1 = value.allocate(0);
        assert!(value.get_mut(generation_index0).is_none());
        assert!(value.get_mut(generation_index1).is_some());
    }

    #[test]
    #[should_panic(expected = "Can only allocate a generation if it's not already allocated")]
    fn test_double_allocate() {
        let mut value = Generation::new();
        value.allocate(0);
        value.allocate(0);
    }

    #[test]
    #[should_panic(expected = "Can only free a generation if it's not already freed")]
    fn test_double_free() {
        let mut value = Generation::new();
        let index = value.allocate(0);

        value.free(index);
        value.free(index);
    }

    #[test]
    #[should_panic(expected = "Can not free a generation with incorrect generation_index")]
    fn test_free_wrong_index() {
        let mut value = Generation::new();
        let index = value.allocate(0);

        value.free(index);
        value.allocate(0);
        value.free(index);
    }
}
