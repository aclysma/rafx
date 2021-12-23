use crossbeam_channel::{Receiver, Sender};
use rafx::api::{
    RafxDeviceContext, RafxExtents3D, RafxFormat, RafxResourceState, RafxResourceType,
    RafxSampleCount, RafxTextureDef, RafxTextureDimensions,
};
use rafx::framework::{ImageViewResource, ResourceArc, ResourceLookupSet};
use rafx::graph::{
    RenderGraphBuilder, RenderGraphExternalImageId, RenderGraphImageExtents,
    RenderGraphImageSpecification,
};
use rafx::RafxResult;

// A raw info struct that can be passed around but does not keep the atlas space allocated
#[derive(Clone, Copy, Debug)]
pub struct ShadowMapAtlasElementInfo {
    pub uv_min: glam::Vec2,
    pub uv_max: glam::Vec2,
}

#[derive(Debug, Clone)]
struct ShadowMapAtlasElementInner {
    uv_min: glam::Vec2,
    uv_max: glam::Vec2,
    texture_size_pixels: u16,
    quality: u8,
}

impl ShadowMapAtlasElementInner {
    fn info(&self) -> ShadowMapAtlasElementInfo {
        ShadowMapAtlasElementInfo {
            uv_min: self.uv_min,
            uv_max: self.uv_max,
        }
    }
}

// An RAII object that keeps the atlas space allocated until it is dropped
pub struct ShadowMapAtlasElement {
    element: Option<ShadowMapAtlasElementInner>,
    drop_tx: Sender<ShadowMapAtlasElementInner>,
}

// When dropped, send elements back to the atlas for reuse
impl Drop for ShadowMapAtlasElement {
    fn drop(&mut self) {
        let element = self.element.take().unwrap();
        self.drop_tx.send(element).unwrap();
    }
}

impl ShadowMapAtlasElement {
    pub fn info(&self) -> ShadowMapAtlasElementInfo {
        self.element.as_ref().unwrap().info()
    }

    pub fn quality(&self) -> u8 {
        self.element.as_ref().unwrap().quality
    }

    pub fn texture_size_pixels(&self) -> u16 {
        self.element.as_ref().unwrap().texture_size_pixels
    }
}

pub struct ShadowMapAtlas {
    _device_context: RafxDeviceContext,
    image_view: ResourceArc<ImageViewResource>,
    free_elements_by_quality: Vec<Vec<ShadowMapAtlasElementInner>>,
    image_width: u32,
    image_height: u32,
    // First render should do a full-clear to get rid of NaN in the image. Call take_requires_full_clear()
    // to check for this condition and clear it.
    requires_full_clear: bool,
    drop_tx: Sender<ShadowMapAtlasElementInner>,
    drop_rx: Receiver<ShadowMapAtlasElementInner>,
}

impl Drop for ShadowMapAtlas {
    fn drop(&mut self) {}
}

impl ShadowMapAtlas {
    pub fn new(resources: &ResourceLookupSet) -> RafxResult<Self> {
        //
        // Configuration:
        //  * atlas_width_height: Size of atlas texture (must be power of 2 and square).
        //  * divisions: We divide the atlas into 4 quadrants. Each quadrant is divided into NxN tiles.
        //
        // This is similar to godot, see their docs:
        // https://docs.godotengine.org/en/stable/tutorials/3d/lights_and_shadows.html
        //
        let atlas_width_height = 4096u32;
        let divisions: [i32; 4] = [2, 4, 4, 8];

        fn create_elements(
            free_elements: &mut [Vec<ShadowMapAtlasElementInner>],
            min_uv: glam::Vec2,
            size_uv: glam::Vec2,
            divisions: u32,
            atlas_width_height: u32,
            texture_size_pixels: u16,
            quality: u8,
        ) {
            for y in 0..divisions {
                for x in 0..divisions {
                    // We add padding to each element to avoid sampling neighboring atlas tiles when
                    // using lienar PCF. (Sampling the black border is ok for non-cubemaps).
                    // For cubemaps, this can cause a seam... we need to use nearest filtering.
                    let inner = ShadowMapAtlasElementInner {
                        uv_min: glam::Vec2::new(
                            min_uv.x
                                + size_uv.x * (x as f32 / divisions as f32)
                                + (1.0 / atlas_width_height as f32),
                            min_uv.y
                                + size_uv.y * (y as f32 / divisions as f32)
                                + (1.0 / atlas_width_height as f32),
                        ),
                        uv_max: glam::Vec2::new(
                            min_uv.x + size_uv.x * ((x + 1) as f32 / divisions as f32)
                                - (1.0 / atlas_width_height as f32),
                            min_uv.y + size_uv.y * ((y + 1) as f32 / divisions as f32)
                                - (1.0 / atlas_width_height as f32),
                        ),
                        texture_size_pixels,
                        quality,
                    };
                    free_elements[quality as usize].push(inner);
                }
            }
        }

        let mut last_divisions = -1;

        let mut free_elements_by_quality = Vec::default();
        for i in 0..4 {
            let divisions = divisions[i as usize];
            assert!(divisions >= last_divisions);
            if divisions != last_divisions {
                free_elements_by_quality.push(Vec::default());
            }

            let quality = free_elements_by_quality.len() - 1;

            create_elements(
                &mut free_elements_by_quality,
                glam::Vec2::new((i % 2) as f32 / 2.0, (i / 2) as f32 / 2.0),
                glam::Vec2::new(0.5, 0.5),
                divisions as u32,
                atlas_width_height,
                (atlas_width_height / 2 / divisions as u32) as u16,
                quality as u8,
            );

            last_divisions = divisions;
        }

        let device_context = resources.device_context().clone();
        let image = device_context.create_texture(&RafxTextureDef {
            format: RafxFormat::D32_SFLOAT,
            resource_type: RafxResourceType::RENDER_TARGET_DEPTH_STENCIL
                | RafxResourceType::TEXTURE,
            extents: RafxExtents3D {
                width: atlas_width_height,
                height: atlas_width_height,
                depth: 1,
            },
            dimensions: RafxTextureDimensions::Auto,
            array_length: 1,
            mip_count: 1,
            sample_count: RafxSampleCount::SampleCount1,
        })?;

        let image = resources.insert_image(image);
        let image_view = resources.get_or_create_image_view(&image, None)?;

        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        Ok(ShadowMapAtlas {
            _device_context: device_context,
            image_view,
            free_elements_by_quality,
            image_width: atlas_width_height,
            image_height: atlas_width_height,
            requires_full_clear: true,
            drop_tx,
            drop_rx,
        })
    }

    pub fn quality_level_count(&self) -> usize {
        self.free_elements_by_quality.len()
    }

    // We do this here instead of in the graph builder so we can keep the atlas init/setup code
    // in sync more easily
    pub fn add_to_render_graph(
        &self,
        graph: &mut RenderGraphBuilder,
    ) -> RenderGraphExternalImageId {
        graph.add_external_image(
            self.shadow_atlas_image_view().clone(),
            RenderGraphImageSpecification {
                samples: RafxSampleCount::SampleCount1,
                format: RafxFormat::D32_SFLOAT,
                resource_type: RafxResourceType::RENDER_TARGET_DEPTH_STENCIL
                    | RafxResourceType::TEXTURE,
                extents: RenderGraphImageExtents::Custom(self.image_width, self.image_height, 1),
                layer_count: 1,
                mip_count: 1,
            },
            Default::default(),
            RafxResourceState::SHADER_RESOURCE,
            RafxResourceState::SHADER_RESOURCE,
        )
    }

    pub fn take_requires_full_clear(&mut self) -> bool {
        let requires_full_clear = self.requires_full_clear;
        self.requires_full_clear = false;
        requires_full_clear
    }

    pub fn shadow_atlas_image_view(&self) -> &ResourceArc<ImageViewResource> {
        &self.image_view
    }

    // Move any release buffers back into the unused_buffers list
    fn handle_dropped_elements(&mut self) {
        for element in self.drop_rx.try_iter() {
            self.free_elements_by_quality[element.quality as usize].push(element);
        }
    }

    pub fn allocate(
        &mut self,
        quality: u8,
    ) -> Option<ShadowMapAtlasElement> {
        self.handle_dropped_elements();

        if let Some(element) = self.free_elements_by_quality[quality as usize].pop() {
            let drop_tx = self.drop_tx.clone();
            return Some(ShadowMapAtlasElement {
                element: Some(element),
                drop_tx,
            });
        } else {
            None
        }
    }
}
