
// Is it possible to use proc macros for registering for passes, entry points, etc?


/// Logic:
/// * Simulate
/// * Compute Views
/// * View Visibility Job(s)
/// * Populate View Render Nodes
/// * Extract Per View Job(s)
/// * Extract Finish
/// * Prepare Per View Job(s)
/// * Publish to Submit
/// * High-Level Submit Script Job(s)
/// * Submit View Job(s)
/// * End Frame Job
/// * Submit Done
///
/// Frame Ring Buffer
/// * Frame Packet
/// * Contains Render Per Frame Nodes
/// * View Packet
/// * Render Per View Nodes
/// * Submit Node Blocks?
/// '
///
///
/// Game object has render components
/// Game objects register with renderer
///  - This produces a render object that may cache static data and/or handles to entity
/// Game object also registers with visibility
///  - This produces a visibility object that has a handle to the render object
/// Entity has a handle to both the render object and the visibility object
/// Extraction will store all data into a ring buffer of frame packets


// struct VisibilityManager {
//     fn register_visibility_type() { }
//
//
// }
//
// struct RenderManager {
//     fn register_render_type() { }
// }





///
/// Render graph can be calculated every frame, it's expected some nodes will not be hooked up and
/// can be skipped. This lets us easily and dynamically switch between them.
///
/// Can also have different views.
///
///
///
///
/// APIs for creating/destroying/updating views
///
/// APIs for creating/destroying render objects
///
/// APIs for creating/destroying visibility objects
///
/// APIs for constructing a render graph
///
/// View contains
///  - Frustum
///  - Node of graph that should be output?
///
/// Frame packet contains view packets
///  - In a view packet, we can create view render nodes that point to render objects
///  - view render nodes are tiny, they might contain type of render object, distance from camera, etc.
///  - they can be sorted by render object type for cache coherency
///  - view render nodes can also allocate frame render nodes - if something is present in multiple views,
///    each view would have their own view render node, but they can share a single frame render node. This
///    is used for caching computation results between views.
///  - data extraction can be batched into jobs. One job for each view/game object type (remember the view render nodes are sorted by type)
///
/// After extraction, we can let the game continue. We will only operate on data we copied into the buffer. This is the prepare phase..
///  * LOD calculations/bucketing can happen here
///  * Non-gameplay logic like particles/cloth can happen
///
/// Submit
///  * Read-only on the frame data
///
/// ** For static data, we can do visibility computation early - we only need controller input and to know where our camera will be.
///    Once we finish ticking the simulation and know where all entity positions will be, we can check visibility for dynamic objects and views
///    - Implies we should be able to fire off a view computation on a subset of data (static stuff) for a particular view early
///
/// * Is this API retained or immediate?
///
/// Feature renderer
///  - Could be a trait with callbacks for extract, prepare, and submit phases?
///
/// NOTE:
/// - I think FIFO priority queue without work-stealing is the most popular async execution strategy for games
///


/// This trait defines all the extension points you can use to extract data from the game, process the data
/// in preparation for submitting to the GPU, and actually doing the submit
///
/// Examples of things you might do in the extract phase:
///  - Calculate and cache transforms for all entities you plan to render
///  - Get additional entity state (like health, ammo count)
///
/// Examples of things you might do in the prepare phase:
///  - Assign/bucket LOD levels for various render objects
///  - Simulate cloth
///
/// Examples of things you might do in the submit phase:
///  - Set render states
///  - Issue individual draw calls
///
/// //Extract/Prepare have the same general outline:
/// // //- begin
/// // - frame_node
/// // - view_nodes
/// // - view_finalize
/// // - frame_finalize
/// //
/// //Submit has this general outline:
/// // - begin
/// // - node
/// // - end
trait FeatureRenderer {

    //
    // Extract
    //

    /// Called once per frame
    fn extract_begin();

    /// Called once per frame/render object
    fn extract_frame_node();

    /// Called once per view/render object
    fn extract_view_nodes();

    /// Called once per view
    fn extract_view_finalize();

    /// Called once per frame
    fn extract_frame_finalize();

    //
    // Prepare
    //

    /// Called once per frame
    fn prepare_begin();

    /// Called once per frame/render object
    fn prepare_frame_node();

    /// Called once per view/render object
    fn prepare_view_nodes();

    /// Called once per view
    fn prepare_view_finalize();

    /// Called once per frame
    fn prepare_frame_finalize();

    //
    // Submit
    //

    /// Called once per submit block
    fn submit_block_begin();

    /// Called once per block node
    fn submit_node();

    /// Called once per submit block
    fn submit_block_end();
}
/*
struct FrameNodeSet;
impl FrameNodeSet{
    fn add_frame_node();
}

struct FrameNode {
    // Entity handle
}

struct ViewNodeSet;
impl ViewNodeSet {
    // Entity handle

    fn add_view_node();
}

struct SubmitNodeSet;
impl SubmitNodeSet {
    // Sort key

    fn add_submit_node();
}

trait RenderStage {
    fn get_sort_key();
}

trait SubmitNodeSortStrategy {

}



fn create_pass_gbuffer() {
    // set_render_targets();
    // set_viewport();
    // submit_render_stage_for_view(first_person_view, render_stage_gbuffer);
}

fn render_graph_node() {
    // NodeBuilder
    //     .add_input_resource(...)
    //     .add_output_resource(...)
}

fn create_view() {
    // View::new(window, viewport, resource, frustum)
}
*/

// use renderer_base::slab::RawSlab;
// use renderer_base::slab::RawSlabKey;
// use glam::{Vec2, Vec3, Vec4, Mat4};



pub mod slab;

pub mod features;

pub mod phases;

pub mod visibility;


mod render_nodes;
pub use render_nodes::*;

mod render_node_set;
pub use render_node_set::RenderNodeSet;

mod render_feature_impl_set;
pub use render_feature_impl_set::RenderFeatureImplSet;

mod render_view;
pub use render_view::RenderView;
pub use render_view::RenderPhaseMaskBuilder;
pub use render_view::RenderPhaseMask;

mod frame_packet;
pub use frame_packet::FramePacket;

mod jobs;

mod registry;
pub use registry::RenderRegistry;
pub use registry::RenderFeature;
pub use registry::RenderPhase;
pub use registry::RenderFeatureIndex;