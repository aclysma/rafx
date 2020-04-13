trait Resource {}

// A feature can use this to request loading resources. The provider is responsible for uploading
// data to the GPU
trait ResourceProvider<T: Resource> {
    fn add_ref_count(
        &self,
        resource_id: u32,
    );
    fn remove_ref_count(
        &self,
        resource_id: u32,
    );

    fn resource(
        &self,
        resource_id: u32,
    ) -> Option<&T>;

    // Need interface that lets us send deltas and repopulate GPU
    // - functions to get added/removed resources?
    // - functions to get all resources?
    // - update call? maybe it produces its own GPU calls?
}

trait ResourceRegistry {
    fn resource_provider<T: Resource>(&self) -> Option<&dyn ResourceProvider<T>>;
}
