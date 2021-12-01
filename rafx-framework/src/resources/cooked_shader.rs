use crate::{
    ComputePipelineResource, FixedFunctionState, MaterialPassResource, ReflectedEntryPoint,
    ReflectedShader, ResourceArc, ResourceLookupSet, ShaderModuleHash,
};
use rafx_api::{RafxResult, RafxShaderPackage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// An import format that will get turned into ShaderAssetData
#[derive(Serialize, Deserialize)]
pub struct CookedShaderPackage {
    pub hash: ShaderModuleHash,
    pub shader_package: RafxShaderPackage,
    pub entry_points: Vec<ReflectedEntryPoint>,
}

impl CookedShaderPackage {
    pub fn find_entry_point(
        &self,
        entry_point_name: &str,
    ) -> Option<&ReflectedEntryPoint> {
        self.entry_points
            .iter()
            .find(|x| x.rafx_api_reflection.entry_point_name == entry_point_name)
    }

    pub fn load_compute_pipeline(
        &self,
        resources: &ResourceLookupSet,
        entry_name: &str,
    ) -> RafxResult<ResourceArc<ComputePipelineResource>> {
        //
        // Find the reflection data in the shader module for the given entry point
        //
        let entry_point = self.find_entry_point(entry_name);
        let entry_point = entry_point.ok_or_else(|| {
            let error_message = format!(
                "Load Compute Shader Failed - Searching for entry point named {}, but no matching reflection data was found",
                entry_name
            );
            log::error!("{}", error_message);
            error_message
        })?;

        let shader_module = resources.get_or_create_shader_module_from_cooked_package(self)?;
        let reflected_shader = ReflectedShader::new(resources, &[shader_module], &[entry_point])?;
        reflected_shader.load_compute_pipeline(resources)
    }

    pub fn load_material_pass(
        resources: &ResourceLookupSet,
        cooked_shader_packages: &[&CookedShaderPackage],
        entry_names: &[&str],
        fixed_function_state: Arc<FixedFunctionState>,
    ) -> RafxResult<ResourceArc<MaterialPassResource>> {
        assert_eq!(cooked_shader_packages.len(), entry_names.len());
        let mut shader_modules = Vec::with_capacity(cooked_shader_packages.len());
        let mut entry_points = Vec::with_capacity(entry_names.len());
        for (cooked_shader_package, entry_name) in cooked_shader_packages.iter().zip(entry_names) {
            shader_modules.push(
                resources
                    .get_or_create_shader_module_from_cooked_package(&cooked_shader_package)?,
            );

            let entry_point = cooked_shader_package.find_entry_point(entry_name);
            let entry_point = entry_point.ok_or_else(|| {
                let error_message = format!(
                    "Load Material Pass Failed - Searching for entry point named {}, but no matching reflection data was found",
                    entry_name
                );
                log::error!("{}", error_message);
                error_message
            })?;

            entry_points.push(entry_point);
        }

        let reflected_shader = ReflectedShader::new(resources, &shader_modules, &entry_points)?;
        reflected_shader.load_material_pass(resources, fixed_function_state)
    }
}
