use super::d3d12;
use crate::dx12::RafxDeviceContextDx12;
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefDx12};
use fnv::{FnvHashMap, FnvHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

fn hash_compile_params(
    entry_point: &str,
    target_profile: &str,
) -> u64 {
    let mut hasher = FnvHasher::default();
    entry_point.hash(&mut hasher);
    target_profile.hash(&mut hasher);
    hasher.finish()
}

pub struct Dx12ShaderBytecodeInner {
    #[allow(unused)]
    bytecode: Vec<u8>,
    dx12_bytecode: d3d12::D3D12_SHADER_BYTECODE,
}

#[derive(Clone)]
pub struct Dx12ShaderBytecode {
    inner: Arc<Dx12ShaderBytecodeInner>,
}

// for d3d12::D3D12_SHADER_BYTECODE
unsafe impl Send for Dx12ShaderBytecode {}
unsafe impl Sync for Dx12ShaderBytecode {}

impl std::fmt::Debug for Dx12ShaderBytecode {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Dx12ShaderBytecode").finish()
    }
}

impl Dx12ShaderBytecode {
    pub fn bytecode(&self) -> &d3d12::D3D12_SHADER_BYTECODE {
        &self.inner.dx12_bytecode
    }
}

#[derive(Debug)]
pub struct RafxShaderModuleDx12Inner {
    dxil_cache: Mutex<FnvHashMap<u64, Dx12ShaderBytecode>>,
    hlsl_src: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleDx12 {
    inner: Arc<RafxShaderModuleDx12Inner>,
}

impl RafxShaderModuleDx12 {
    // pub fn library(&self) -> &metal_rs::LibraryRef {
    //     self.inner.library.as_ref()
    // }

    pub fn get_or_compile_bytecode(
        &self,
        entry_point: &str,
        target_profile: &str,
    ) -> RafxResult<Dx12ShaderBytecode> {
        let compile_params_hash = hash_compile_params(entry_point, target_profile);

        let mut dxil_cache = self.inner.dxil_cache.lock().unwrap();
        if let Some(cached_dxil) = dxil_cache.get(&compile_params_hash) {
            return Ok(cached_dxil.clone());
        }

        // We don't have bytecode available, need to compile it
        let src = self.inner.hlsl_src.as_ref().unwrap();
        let mut bytecode = hassle_rs::compile_hlsl(
            "shader.hlsl",
            src,
            entry_point,
            target_profile,
            &["/Zi"],
            &[],
        )?;

        hassle_rs::fake_sign_dxil_in_place(&mut bytecode);

        let dx12_bytecode = d3d12::D3D12_SHADER_BYTECODE {
            pShaderBytecode: &bytecode[0] as *const u8 as *const std::ffi::c_void,
            BytecodeLength: bytecode.len(),
        };

        let inner = Dx12ShaderBytecodeInner {
            bytecode,
            dx12_bytecode,
        };

        let shader_bytecode = Dx12ShaderBytecode {
            inner: Arc::new(inner),
        };

        dxil_cache.insert(compile_params_hash, shader_bytecode.clone());

        return Ok(shader_bytecode);
    }

    pub fn new(
        _device_context: &RafxDeviceContextDx12,
        data: RafxShaderModuleDefDx12,
    ) -> RafxResult<Self> {
        match data {
            // RafxShaderModuleDefDx12::Dxil(dxil) => {
            //     RafxShaderModuleDx12::new_from_dxil(device_context, bytes)
            // }
            RafxShaderModuleDefDx12::HlslSrc(src) => {
                RafxShaderModuleDx12::new_from_src(src.to_string())
            }
        }
    }

    // pub fn new_from_dxil(
    //     device_context: &RafxDeviceContextDx12,
    //     dxil: &[u8],
    // ) -> RafxResult<Self> {
    //     // let library = device_context.device().new_library_with_data(data)?;
    //     //
    //     // let inner = RafxShaderModuleDx12Inner { library };
    //     //
    //     // Ok(RafxShaderModuleDx12 {
    //     //     inner: Arc::new(inner),
    //     // })
    //     unimplemented!()
    // }

    pub fn new_from_src(src: String) -> RafxResult<Self> {
        let inner = RafxShaderModuleDx12Inner {
            dxil_cache: Default::default(),
            hlsl_src: Some(src),
        };

        Ok(RafxShaderModuleDx12 {
            inner: Arc::new(inner),
        })
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleDx12 {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Dx12(self)
    }
}
