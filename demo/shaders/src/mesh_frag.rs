#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct SpotLightStd140 {
    pub position_ws: [f32; 3],          // +0 (size: 12)
    pub direction_ws: [f32; 3],         // +12 (size: 12)
    pub position_vs: [f32; 3],          // +24 (size: 12)
    pub direction_vs: [f32; 3],         // +36 (size: 12)
    pub color: [f32; 4],                // +48 (size: 16)
    pub spotlight_half_angle: f32,      // +64 (size: 4)
    pub range: f32,                     // +68 (size: 4)
    pub intensity: f32,                 // +72 (size: 4)
} // 96 bytes

pub type SpotLightUniform = SpotLightStd140;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct DirectionalLightStd140 {
    pub direction_ws: [f32; 3],         // +0 (size: 12)
    pub direction_vs: [f32; 3],         // +12 (size: 12)
    pub color: [f32; 4],                // +24 (size: 16)
    pub intensity: f32,                 // +40 (size: 4)
} // 64 bytes

pub type DirectionalLightUniform = DirectionalLightStd140;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct PointLightStd140 {
    pub position_ws: [f32; 3],          // +0 (size: 12)
    pub position_vs: [f32; 3],          // +12 (size: 12)
    pub color: [f32; 4],                // +24 (size: 16)
    pub range: f32,                     // +40 (size: 4)
    pub intensity: f32,                 // +44 (size: 4)
} // 64 bytes

pub type PointLightUniform = PointLightStd140;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct PerViewDataStd140 {
    pub ambient_light: [f32; 4],        // +0 (size: 16)
    pub point_light_count: u32,         // +16 (size: 4)
    pub directional_light_count: u32,   // +20 (size: 4)
    pub spot_light_count: u32,          // +24 (size: 4)
    pub point_lights: [PointLightStd140; 16], // +28 (size: 1024)
    pub directional_lights: [DirectionalLightStd140; 16], // +1052 (size: 1024)
    pub spot_lights: [SpotLightStd140; 16], // +2076 (size: 1536)
} // 3616 bytes

pub type PerViewDataUniform = PerViewDataStd140;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_struct_SpotLightStd140() {
        assert_eq!(std::mem::size_of::<SpotLightStd140>(), 96);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, position_ws), 0);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, direction_ws), 12);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, position_vs), 24);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, direction_vs), 36);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, color), 48);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, spotlight_half_angle), 64);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, range), 68);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(SpotLightStd140, intensity), 72);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_struct_DirectionalLightStd140() {
        assert_eq!(std::mem::size_of::<DirectionalLightStd140>(), 64);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, direction_ws), 0);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, direction_vs), 12);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, color), 24);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, intensity), 40);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_struct_PointLightStd140() {
        assert_eq!(std::mem::size_of::<PointLightStd140>(), 64);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(PointLightStd140, position_ws), 0);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(memoffset::offset_of!(PointLightStd140, position_vs), 12);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(memoffset::offset_of!(PointLightStd140, color), 24);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(PointLightStd140, range), 40);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(PointLightStd140, intensity), 44);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_struct_PerViewDataStd140() {
        assert_eq!(std::mem::size_of::<PerViewDataStd140>(), 3616);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, ambient_light), 0);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, point_light_count), 16);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, directional_light_count), 20);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, spot_light_count), 24);
        assert_eq!(std::mem::size_of::<[PointLightStd140; 16]>(), 1024);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, point_lights), 28);
        assert_eq!(std::mem::size_of::<[DirectionalLightStd140; 16]>(), 1024);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, directional_lights), 1052);
        assert_eq!(std::mem::size_of::<[SpotLightStd140; 16]>(), 1536);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, spot_lights), 2076);
    }
}
