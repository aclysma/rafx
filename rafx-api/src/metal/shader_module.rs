use crate::RafxResult;

#[derive(Clone, Debug)]
pub struct RafxShaderModuleMetal {
    library: metal::Library,
}

unsafe impl Send for RafxShaderModuleMetal {}
unsafe impl Sync for RafxShaderModuleMetal {}

impl RafxShaderModuleMetal {
    pub fn new_from_source(
        device: &metal::Device,
        source: &str,
        compile_options: &metal::CompileOptions,
    ) -> RafxResult<Self> {
        let library = device.new_library_with_source(source, compile_options)?;
        Ok(RafxShaderModuleMetal { library })
    }

    pub fn new_from_library_file<P: AsRef<std::path::Path>>(
        device: &metal::Device,
        file: P,
    ) -> RafxResult<Self> {
        let library = device.new_library_with_file(file)?;
        Ok(RafxShaderModuleMetal { library })
    }

    pub fn library(&self) -> &metal::Library {
        &self.library
    }
}
