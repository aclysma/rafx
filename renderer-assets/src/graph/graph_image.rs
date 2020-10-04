use ash::vk;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageVersionId {
    pub(super) index: usize,
    pub(super) version: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderGraphImageUsageId(pub(super) usize);

#[derive(Debug)]
pub struct RenderGraphImageUsage {
    pub(super) usage_type: RenderGraphImageUsageType,
    pub(super) version: RenderGraphImageVersionId,
}

pub type RenderGraphResourceName = &'static str;

//
// The state of an image
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphImageSpecification {
    //pub layout: vk::ImageLayout,
    pub samples: vk::SampleCountFlags,
    pub format: vk::Format,
    pub queue: u32,
}

//
// Constraints on an image. Constraints are set per-field
//
#[derive(Default, Clone, Debug)]
pub struct RenderGraphImageConstraint {
    //pub layout: Option<vk::ImageLayout>,
    pub samples: Option<vk::SampleCountFlags>,
    pub format: Option<vk::Format>,
    pub queue: Option<u32>, // format? size?
}

impl From<RenderGraphImageSpecification> for RenderGraphImageConstraint {
    fn from(specification: RenderGraphImageSpecification) -> Self {
        RenderGraphImageConstraint {
            //layout: Some(specification.layout),
            samples: Some(specification.samples),
            format: Some(specification.format),
            queue: Some(specification.queue),
        }
    }
}

impl RenderGraphImageConstraint {
    pub fn try_convert_to_specification(self) -> Option<RenderGraphImageSpecification> {
        if self.samples.is_none() || self.format.is_none() || self.queue.is_none() {
            None
        } else {
            Some(RenderGraphImageSpecification {
                samples: self.samples.unwrap(),
                format: self.format.unwrap(),
                queue: self.queue.unwrap(),
            })
        }
    }
}

impl RenderGraphImageConstraint {
    pub fn can_merge(
        &self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        // if self.layout.is_some() && other.layout.is_some() && self.layout != other.layout {
        //     return false;
        // }
        if self.samples.is_some() && other.samples.is_some() && self.samples != other.samples {
            return false;
        }
        if self.format.is_some() && other.format.is_some() && self.format != other.format {
            return false;
        }
        if self.queue.is_some() && other.queue.is_some() && self.queue != other.queue {
            return false;
        }

        true
    }

    pub fn try_merge(
        &mut self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        // if self.layout.is_none() {
        //     self.layout = other.layout;
        // }
        if self.samples.is_none() && other.samples.is_some() {
            self.samples = other.samples;
        }
        if self.format.is_none() && other.format.is_some() {
            self.format = other.format;
        }
        if self.queue.is_none() && other.queue.is_some() {
            self.queue = other.queue;
        }

        true
    }

    pub fn partial_merge(
        &mut self,
        other: &RenderGraphImageConstraint,
    ) -> bool {
        let mut complete_merge = true;
        // if self.layout.is_some() && other.layout.is_some() && self.layout != other.layout {
        //     complete_merge = false;
        // } else {
        //     self.layout = other.layout;
        // }

        if self.samples.is_some() && other.samples.is_some() && self.samples != other.samples {
            complete_merge = false;
        } else if other.samples.is_some() {
            self.samples = other.samples;
        }

        if self.format.is_some() && other.format.is_some() && self.format != other.format {
            complete_merge = false;
        } else if other.format.is_some() {
            self.format = other.format;
        }

        if self.queue.is_some() && other.queue.is_some() && self.queue != other.queue {
            complete_merge = false;
        } else if other.queue.is_some() {
            self.queue = other.queue;
        }

        complete_merge
    }

    pub fn set(
        &mut self,
        other: &RenderGraphImageSpecification,
    ) {
        *self = other.clone().into();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RenderGraphImageUsageType {
    Create,
    Input,
    Read,
    ModifyRead,
    ModifyWrite,
    Output,
}

impl RenderGraphImageUsageType {
    pub fn is_read_only(&self) -> bool {
        match self {
            RenderGraphImageUsageType::Read => true,
            RenderGraphImageUsageType::Output => true,
            RenderGraphImageUsageType::ModifyRead => false,
            RenderGraphImageUsageType::Create => false,
            RenderGraphImageUsageType::Input => false,
            RenderGraphImageUsageType::ModifyWrite => false,
        }
    }
}

#[derive(Debug)]
pub struct RenderGraphImageResourceVersionInfo {
    pub(super) creator_node: RenderGraphNodeId,

    pub(super) create_usage: RenderGraphImageUsageId,
    pub(super) read_usages: Vec<RenderGraphImageUsageId>
}

impl RenderGraphImageResourceVersionInfo {
    pub(super) fn new(creator: RenderGraphNodeId, create_usage: RenderGraphImageUsageId) -> Self {
        RenderGraphImageResourceVersionInfo {
            creator_node: creator,
            //usages: Default::default(),
            create_usage,
            read_usages: Default::default()
        }
    }

    pub(super) fn remove_read_usage(&mut self, usage: RenderGraphImageUsageId) {
        if let Some(position) = self.read_usages.iter().position(|x| *x == usage) {
            self.read_usages.swap_remove(position);
        }
    }

    pub(super) fn add_read_usage(&mut self, usage: RenderGraphImageUsageId) {
        self.read_usages.push(usage);
    }
}

//
// A "virtual" image that the render graph knows about. The render graph will allocate images as
// needed, but can reuse the same image for multiple resources if the lifetimes of those images
// don't overlap
//
#[derive(Debug)]
pub struct RenderGraphImageResource {
    pub(super) name: Option<RenderGraphResourceName>,

    pub(super) versions: Vec<RenderGraphImageResourceVersionInfo>,
}

impl RenderGraphImageResource {
    pub(super) fn new() -> Self {
        RenderGraphImageResource {
            name: None,
            versions: Default::default(),
        }
    }
}

//
// A helper for configuring an image. This helper allows us to have a borrow against the rest of
// the graph data, allowing us to write data into nodes as well as images
//
pub struct RenderGraphImageResourceConfigureContext<'a> {
    pub(super) graph: &'a mut RenderGraph,
    pub(super) image_id: RenderGraphImageUsageId,
}

impl<'a> RenderGraphImageResourceConfigureContext<'a> {
    pub fn id(&self) -> RenderGraphImageUsageId {
        self.image_id
    }

    pub fn set_name(
        self,
        name: RenderGraphResourceName,
    ) -> Self {
        self.graph.image_resource_mut(self.image_id).name = Some(name);
        self
    }

    /*
        // Ties an image to the intial state of an image resource. The graph may use the image directly
        // to execute some nodes. In other words, the graph will take ownership of the image and leave
        // it in an undefined state.
        pub fn set_input_image(
            &mut self,
            src_image: (), /*ResourceArc<ImageViewResource>*/
            state: RenderGraphImageSpecification,
        ) -> &mut Self {
            let image_version = self.graph.image_version_info_by_usage(self.image_id);
            let usage = image_version.usages.len();

            let version_id = RenderGraphImageVersionId {
                index: self.image_id.index,
                version: self.image_id.version
            };
            let usage_id = self.graph.add_usage(version_id, RenderGraphImageUsageType::Input, usage);

            let mut image_version = self.graph.image_version_info_by_usage_mut(self.image_id);
            image_version
                .usages
                .push(RenderGraphImageResourceUsage::new(
                    RenderGraphImageUsageType::Input,
                    usage_id
                ));

            image_version.input_image = Some(src_image);
            image_version.input_specification = Some(state);

            self.graph.input_images.push(usage_id);
            self
        }
    */

    // Ties an image to the final state of an image resource. After the graph executes, the result
    // will be placed into this image. The graph may use the image directly to execute some nodes.
    // In other words, the graph will take ownership of the image and leave it in whatever state
    // the bound resource would have been left in.
    pub fn set_output_image(
        &mut self,
        dst_image: (), /*ResourceArc<ImageViewResource>*/
        state: RenderGraphImageSpecification,
    ) -> &mut Self {
        let version_id = self.graph.image_version_id(self.image_id);
        let usage_id = self
            .graph
            .add_usage(version_id, RenderGraphImageUsageType::Output);

        let mut image_version = self.graph.image_version_info_mut(self.image_id);
        image_version
            .read_usages
            .push(usage_id);

        let output_image = RenderGraphOutputImage {
            usage: usage_id,
            specification: state
        };

        self.graph.output_images.push(output_image);
        self
    }
}
