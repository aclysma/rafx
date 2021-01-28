# Using Generated Rust Code

Using `@[export]` will generate Rust code that matches the shader. This avoids many easy-to-make mistakes caused by
Rust structs that don't perfectly align with shader code. This is particularly tricky because there are multiple
alignment rules in shader code (GLSL calls them `std140`/`std430`) and the correct one depends on how the struct is 
used! In fact, it's possible for a single struct in the same shader to be used with BOTH layouts, depending on where it 
is being read from!

Symbol names are inferred from the shader code, but may be overriden with `@[slot_name("custom_name_here)]`. This also
affects reflection data names and material parameter names. Overriding the name allows shader variables to be renamed
without breaking dependent rust code or other references to the name that might be stored in asset data.

## Example

In this case, we'll export a uniform variable.

```c
// This is automatically exported because it is referenced in PerViewData, which is exported
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;
    int shadow_map;
};

// A Std140-compatible struct will be exported for this since it's a uniform variable
// @[export]
layout (set = 0, binding = 0) uniform PerViewData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
} per_view_data;
```

This will cause the shader processor to generate rust code for these structs

```rust
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PointLightStd140 {
    pub position_ws: [f32; 3], // +0 (size: 12)
    pub _padding0: [u8; 4],    // +12 (size: 4)
    pub position_vs: [f32; 3], // +16 (size: 12)
    pub _padding1: [u8; 4],    // +28 (size: 4)
    pub color: [f32; 4],       // +32 (size: 16)
    pub range: f32,            // +48 (size: 4)
    pub intensity: f32,        // +52 (size: 4)
    pub shadow_map: i32,       // +56 (size: 4)
    pub _padding2: [u8; 4],    // +60 (size: 4)
} // 64 bytes

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PerViewDataStd140 {
    pub ambient_light: [f32; 4],                             // +0 (size: 16)
    pub point_light_count: u32,                              // +16 (size: 4)
    pub _padding0: [u8; 4],                                  // +28 (size: 4)
    pub point_lights: [PointLightStd140; 16],                // +32 (size: 1024)
    pub shadow_map_cube_data: [ShadowMapCubeDataStd140; 16], // +6176 (size: 256)
} // 6432 bytes

pub type PerViewDataUniform = PerViewDataStd140;
```

In addition, a descriptor set struct will be exported that integrates with `rafx-framework`'s descriptor set management.

```rust
pub struct DescriptorSet0(pub DynDescriptorSet);

impl DescriptorSet0 {
    pub fn set_args_static(
        descriptor_set: &mut DynDescriptorSet,
        args: DescriptorSet0Args,
    ) {
        descriptor_set.set_buffer_data(
            PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            args.per_view_data,
        );
        descriptor_set.set_images(
            SHADOW_MAP_IMAGES_CUBE_DESCRIPTOR_BINDING_INDEX as u32,
            args.shadow_map_images_cube,
        );
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_per_view_data(args.per_view_data);
        self.set_shadow_map_images_cube(args.shadow_map_images_cube);
    }

    pub fn set_per_view_data(
        &mut self,
        per_view_data: &PerViewDataUniform,
    ) {
        self.0
            .set_buffer_data(PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32, per_view_data);
    }

    pub fn set_shadow_map_images_cube(
        &mut self,
        shadow_map_images_cube: &[Option<&ResourceArc<ImageViewResource>>; 16],
    ) {
        self.0.set_images(
            SHADOW_MAP_IMAGES_CUBE_DESCRIPTOR_BINDING_INDEX as u32,
            shadow_map_images_cube,
        );
    }

    pub fn set_shadow_map_images_cube_element(
        &mut self,
        index: usize,
        element: &ResourceArc<ImageViewResource>,
    ) {
        self.0.set_image_at_index(
            SHADOW_MAP_IMAGES_CUBE_DESCRIPTOR_BINDING_INDEX as u32,
            index,
            element,
        );
    }

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> RafxResult<()> {
        self.0.flush(descriptor_set_allocator)
    }
}
```

Creating that descriptor set might look like this:

```rust
let descriptor_set = descriptor_set_allocator.create_descriptor_set(
    &per_view_descriptor_set_layout,
    DescriptorSet0Args {
        shadow_map_images_cube: &shadow_map_cube_image_views,
        per_view_data: &PerViewDataUniform {
            // set fields for PerViewDataUniform here!
            ambient_light: [0.02, 0.02, 0.02, 0.02],
            ..Default::default()
        },
    },
).unwrap();
```
