mod resource_manager_init;
pub use resource_manager_init::create_resource_manager;

//TODO: Collapse resource_managers into the root of this crate
pub mod resource_managers;
pub use resource_managers::*;
