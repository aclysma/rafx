use crate::{
    RafxApiDefDx12, RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef,
    RafxDeviceContext, RafxDeviceInfo, RafxDrawIndexedIndirectCommand, RafxDrawIndirectCommand,
    RafxError, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType, RafxResourceType, RafxResult,
    RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef, RafxShaderModuleDefDx12,
    RafxShaderStageDef, RafxSwapchainDef, RafxTextureDef, RafxValidationMode,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use crate::dx12::{
    RafxBufferDx12, RafxDescriptorSetArrayDx12, RafxDx12FeatureLevel, RafxFenceDx12,
    RafxPipelineDx12, RafxQueueDx12, RafxRootSignatureDx12, RafxSamplerDx12, RafxSemaphoreDx12,
    RafxShaderDx12, RafxShaderModuleDx12, RafxSwapchainDx12, RafxTextureDx12,
};

use super::d3d;
use super::d3d12;
use super::dxgi;

fn wchar_to_string(s: &[u16]) -> String {
    let wchar = s.split(|&v| v == 0).next().unwrap();
    String::from_utf16_lossy(wchar)
}

fn get_hardware_adapter(
    factory: &dxgi::IDXGIFactory4,
    dx12_api_def: &RafxApiDefDx12,
) -> RafxResult<dxgi::IDXGIAdapter1> {
    let minimum_feature_level = match dx12_api_def.minimum_feature_level {
        RafxDx12FeatureLevel::Dx12FeatureLevel_12_0 => d3d::D3D_FEATURE_LEVEL_12_0,
        RafxDx12FeatureLevel::Dx12FeatureLevel_12_1 => d3d::D3D_FEATURE_LEVEL_12_1,
        RafxDx12FeatureLevel::Dx12FeatureLevel_12_2 => d3d::D3D_FEATURE_LEVEL_12_2,
    };

    //TODO: Select the best device (using memory or feature support)
    for i in 0.. {
        // Returns DXGI_ERROR_NOT_FOUND if we run out of adapters to check
        let adapter = unsafe { factory.EnumAdapters1(i)? };

        let mut desc = Default::default();
        unsafe { adapter.GetDesc1(&mut desc)? };

        let a = wchar_to_string(&desc.Description);
        log::info!("Found device {:?}", a);
        log::info!(
            "  Vendor Id:{} Device Id: {} SubSysId: {} Revision: {}",
            desc.VendorId,
            desc.DeviceId,
            desc.SubSysId,
            desc.Revision
        );
        log::info!(
            "  Dedicated VMem: {} Dedicated System Mem: {} Shared System Mem: {}",
            desc.DedicatedVideoMemory,
            desc.DedicatedSystemMemory,
            desc.SharedSystemMemory
        );

        if (dxgi::DXGI_ADAPTER_FLAG(desc.Flags) & dxgi::DXGI_ADAPTER_FLAG_SOFTWARE)
            != dxgi::DXGI_ADAPTER_FLAG_NONE
        {
            // Don't select the Basic Render Driver adapter. If you want a
            // software adapter, set use_warp_device to true in RafxApiDefDx12
            continue;
        }

        // Check to see whether the adapter supports Direct3D 12, but don't
        // create the actual device yet.
        if unsafe {
            d3d12::D3D12CreateDevice(
                &adapter,
                minimum_feature_level,
                std::ptr::null_mut::<Option<d3d12::ID3D12Device>>(),
            )
        }
        .is_ok()
        {
            return Ok(adapter);
        }
    }

    unreachable!()
}

fn create_indirect_draw_command_signature(
    device: &d3d12::ID3D12Device,
    indexed: bool,
) -> RafxResult<d3d12::ID3D12CommandSignature> {
    let mut sig = d3d12::D3D12_COMMAND_SIGNATURE_DESC::default();
    let mut arg = d3d12::D3D12_INDIRECT_ARGUMENT_DESC::default();

    if !indexed {
        arg.Type = d3d12::D3D12_INDIRECT_ARGUMENT_TYPE_DRAW;
        sig.ByteStride = std::mem::size_of::<RafxDrawIndirectCommand>() as u32;
    } else {
        arg.Type = d3d12::D3D12_INDIRECT_ARGUMENT_TYPE_DRAW_INDEXED;
        sig.ByteStride = std::mem::size_of::<RafxDrawIndexedIndirectCommand>() as u32;
    }

    sig.NumArgumentDescs = 1;
    sig.pArgumentDescs = &arg;

    let mut result: Option<d3d12::ID3D12CommandSignature> = None;

    unsafe {
        device.CreateCommandSignature(&sig, None, &mut result)?;
    }

    Ok(result.unwrap())
}

fn create_device(
    dx12_api_def: &RafxApiDefDx12
) -> RafxResult<(
    dxgi::IDXGIFactory4,
    dxgi::IDXGIAdapter1,
    d3d12::ID3D12Device,
)> {
    if dx12_api_def.validation_mode != RafxValidationMode::Disabled {
        unsafe {
            let mut debug: Option<d3d12::ID3D12Debug> = None;
            if let Some(debug) = d3d12::D3D12GetDebugInterface(&mut debug).ok().and(debug) {
                debug.EnableDebugLayer();
                let debug1: d3d12::ID3D12Debug1 = debug.cast().unwrap();
                debug1.SetEnableGPUBasedValidation(dx12_api_def.enable_gpu_based_validation);
            } else {
                if dx12_api_def.validation_mode == RafxValidationMode::EnabledIfAvailable {
                    log::warn!("Could not acquire D3D12GetDebugInterface.");
                } else {
                    // Fail initialization
                    log::error!("Could not acquire D3D12GetDebugInterface.");
                    return Err(RafxError::ValidationRequiredButUnavailable);
                }
            }
        }
    }

    let dxgi_factory_flags = if dx12_api_def.validation_mode != RafxValidationMode::Disabled {
        dxgi::DXGI_CREATE_FACTORY_DEBUG
    } else {
        0
    };

    let dxgi_factory: dxgi::IDXGIFactory4 =
        unsafe { dxgi::CreateDXGIFactory2(dxgi_factory_flags) }?;

    let dxgi_adapter = if dx12_api_def.use_warp_device {
        unsafe {
            log::info!("Creating warp adapter");
            dxgi_factory
                .EnumWarpAdapter()
                .map_err(|e| RafxError::WindowsApiError(e))
        }
    } else {
        get_hardware_adapter(&dxgi_factory, dx12_api_def)
    }?;

    let mut device: Option<d3d12::ID3D12Device> = None;
    unsafe { d3d12::D3D12CreateDevice(&dxgi_adapter, d3d::D3D_FEATURE_LEVEL_11_0, &mut device) }?;

    let d3d12_device = device.ok_or(RafxError::StringError(
        "Could not create D3D device".to_string(),
    ))?;

    if dx12_api_def.validation_mode != RafxValidationMode::Disabled {
        let info_queue = d3d12_device.cast::<d3d12::ID3D12InfoQueue>().unwrap();
        unsafe {
            info_queue.SetBreakOnSeverity(d3d12::D3D12_MESSAGE_SEVERITY_ERROR, true)?;
            //info_queue.SetBreakOnSeverity(d3d12::D3D12_MESSAGE_SEVERITY_WARNING, true)?;
            info_queue.SetBreakOnSeverity(d3d12::D3D12_MESSAGE_SEVERITY_CORRUPTION, true)?;
        }

        // Set what degree of GBV we want to use.
        if dx12_api_def.enable_gpu_based_validation {
            // this cast only works if we enabled validation earlier
            // https://gamedev.net/forums/topic/672268-d3d12-debug-layers-how-to-get-id3d12debugdevice/5255763/
            let debug_device = d3d12_device.cast::<d3d12::ID3D12DebugDevice1>()?;
            let gbv_settings = d3d12::D3D12_DEBUG_DEVICE_GPU_BASED_VALIDATION_SETTINGS {
                MaxMessagesPerCommandList: 0,
                // Interesting options for GPU-based validation
                // D3D12_GPU_BASED_VALIDATION_SHADER_PATCH_MODE_STATE_TRACKING_ONLY,
                // D3D12_GPU_BASED_VALIDATION_SHADER_PATCH_MODE_UNGUARDED_VALIDATION,
                // D3D12_GPU_BASED_VALIDATION_SHADER_PATCH_MODE_GUARDED_VALIDATION,
                DefaultShaderPatchMode:
                    d3d12::D3D12_GPU_BASED_VALIDATION_SHADER_PATCH_MODE_UNGUARDED_VALIDATION,
                PipelineStateCreateFlags:
                    d3d12::D3D12_GPU_BASED_VALIDATION_PIPELINE_STATE_CREATE_FLAG_NONE,
            };
            unsafe {
                debug_device.SetDebugParameter(
                    d3d12::D3D12_DEBUG_DEVICE_PARAMETER_GPU_BASED_VALIDATION_SETTINGS,
                    &gbv_settings as *const d3d12::D3D12_DEBUG_DEVICE_GPU_BASED_VALIDATION_SETTINGS
                        as *const std::ffi::c_void,
                    std::mem::size_of::<d3d12::D3D12_DEBUG_DEVICE_GPU_BASED_VALIDATION_SETTINGS>()
                        as u32,
                )?;
            }
        }
    }

    Ok((dxgi_factory, dxgi_adapter, d3d12_device))
}

use crate::dx12::mipmap_resources::Dx12MipmapResources;
#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::Interface;

pub struct RafxDeviceContextDx12Inner {
    pub(crate) device_info: RafxDeviceInfo,

    allocator: ManuallyDrop<Mutex<gpu_allocator::d3d12::Allocator>>,

    pub(crate) indirect_command_signature: d3d12::ID3D12CommandSignature,
    pub(crate) indirect_command_signature_indexed: d3d12::ID3D12CommandSignature,

    d3d12_device: d3d12::ID3D12Device,
    dxgi_adapter: dxgi::IDXGIAdapter1,
    dxgi_factory: dxgi::IDXGIFactory4,

    pub(crate) heaps: super::internal::descriptor_heap::Dx12DescriptorHeapSet,

    pub(crate) mipmap_resources: rafx_base::trust_cell::TrustCell<Option<Dx12MipmapResources>>,

    destroyed: AtomicBool,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

// For metal_rs::Device
unsafe impl Send for RafxDeviceContextDx12Inner {}
unsafe impl Sync for RafxDeviceContextDx12Inner {}

impl Drop for RafxDeviceContextDx12Inner {
    fn drop(&mut self) {
        log::trace!("destroying device");

        // We expect this is already set to None by RafxApiDx12::destroy so that there are no
        // remaining references to RafxDeviceContextDx12
        assert!(self.mipmap_resources.borrow().is_none());

        if !self.destroyed.swap(true, Ordering::AcqRel) {
            unsafe {
                log::trace!("destroying device");
                self.allocator
                    .lock()
                    .unwrap()
                    .report_memory_leaks(log::Level::Warn);
                ManuallyDrop::drop(&mut self.allocator);
            }
        }
    }
}

impl RafxDeviceContextDx12Inner {
    pub fn new(dx12_api_def: &RafxApiDefDx12) -> RafxResult<Self> {
        let (dxgi_factory, dxgi_adapter, d3d12_device) = create_device(dx12_api_def)?;

        let mut desc = Default::default();
        unsafe { dxgi_adapter.GetDesc1(&mut desc)? };

        let heaps = super::internal::descriptor_heap::Dx12DescriptorHeapSet::new(&d3d12_device)?;

        let allocator_create_info = gpu_allocator::d3d12::AllocatorCreateDesc {
            device: d3d12_device.clone(),
            debug_settings: Default::default(),
        };

        let allocator = gpu_allocator::d3d12::Allocator::new(&allocator_create_info)?;

        let indirect_command_signature =
            create_indirect_draw_command_signature(&d3d12_device, false)?;
        let indirect_command_signature_indexed =
            create_indirect_draw_command_signature(&d3d12_device, true)?;

        let device_info = RafxDeviceInfo {
            supports_multithreaded_usage: true,
            debug_names_enabled: dx12_api_def.enable_debug_names,
            // pretty sure this is consistent across macOS device (maybe not M1, not sure)
            min_uniform_buffer_offset_alignment:
                d3d12::D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT,
            // based on one of the loosest vulkan limits (intel iGPU), can't find official value
            min_storage_buffer_offset_alignment: 64,
            upload_texture_alignment: d3d12::D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT,
            upload_texture_row_alignment: d3d12::D3D12_TEXTURE_DATA_PITCH_ALIGNMENT,
            supports_clamp_to_border_color: true,
            max_vertex_attribute_count: 31,
        };

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        Ok(RafxDeviceContextDx12Inner {
            device_info,

            allocator: ManuallyDrop::new(Mutex::new(allocator)),

            indirect_command_signature,
            indirect_command_signature_indexed,

            d3d12_device,
            dxgi_adapter,
            dxgi_factory,

            heaps,

            mipmap_resources: Default::default(),

            destroyed: AtomicBool::new(false),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            all_contexts: Mutex::new(all_contexts),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            next_create_index: AtomicU64::new(1),
        })
    }
}

pub struct RafxDeviceContextDx12 {
    pub(crate) inner: Arc<RafxDeviceContextDx12Inner>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) create_index: u64,
}

impl std::fmt::Debug for RafxDeviceContextDx12 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDeviceContextDx12").finish()
    }
}

impl Clone for RafxDeviceContextDx12 {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let create_index = {
            let create_index = self.inner.next_create_index.fetch_add(1, Ordering::Relaxed);

            #[cfg(feature = "track-device-contexts")]
            {
                let create_backtrace = backtrace::Backtrace::new_unresolved();
                self.inner
                    .as_ref()
                    .all_contexts
                    .lock()
                    .unwrap()
                    .insert(create_index, create_backtrace);
            }

            log::trace!("Cloned RafxDeviceContextDx12 create_index {}", create_index);
            create_index
        };

        RafxDeviceContextDx12 {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl Drop for RafxDeviceContextDx12 {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        {
            self.inner
                .all_contexts
                .lock()
                .unwrap()
                .remove(&self.create_index);
        }
    }
}

impl Into<RafxDeviceContext> for RafxDeviceContextDx12 {
    fn into(self) -> RafxDeviceContext {
        RafxDeviceContext::Dx12(self)
    }
}

impl RafxDeviceContextDx12 {
    pub fn device_info(&self) -> &RafxDeviceInfo {
        &self.inner.device_info
    }

    pub fn dxgi_factory(&self) -> &dxgi::IDXGIFactory4 {
        &self.inner.dxgi_factory
    }

    pub fn dxgi_adapter(&self) -> &dxgi::IDXGIAdapter1 {
        &self.inner.dxgi_adapter
    }

    pub fn d3d12_device(&self) -> &d3d12::ID3D12Device {
        &self.inner.d3d12_device
    }

    pub fn allocator(&self) -> &Mutex<gpu_allocator::d3d12::Allocator> {
        &self.inner.allocator
    }

    pub fn new(inner: Arc<RafxDeviceContextDx12Inner>) -> RafxResult<Self> {
        let dx12_device_context = RafxDeviceContextDx12 {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        };

        let mipmap_resources = Dx12MipmapResources::new(&dx12_device_context)?;
        *dx12_device_context.inner.mipmap_resources.borrow_mut() = Some(mipmap_resources);

        Ok(dx12_device_context)
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueDx12> {
        RafxQueueDx12::new(self, queue_type)
    }

    pub fn create_fence(&self) -> RafxResult<RafxFenceDx12> {
        RafxFenceDx12::new(self)
    }

    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreDx12> {
        RafxSemaphoreDx12::new(self)
    }

    pub fn create_swapchain(
        &self,
        raw_display_handle: &dyn HasRawDisplayHandle,
        raw_window_handle: &dyn HasRawWindowHandle,
        present_queue: &RafxQueueDx12,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainDx12> {
        RafxSwapchainDx12::new(
            self,
            raw_display_handle,
            raw_window_handle,
            swapchain_def,
            present_queue,
        )
    }

    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFenceDx12],
    ) -> RafxResult<()> {
        RafxFenceDx12::wait_for_fences(self, fences)
    }

    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerDx12> {
        RafxSamplerDx12::new(self, sampler_def)
    }

    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureDx12> {
        RafxTextureDx12::new(self, texture_def)
    }

    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBufferDx12> {
        RafxBufferDx12::new(self, buffer_def)
    }

    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShaderDx12> {
        RafxShaderDx12::new(self, stages)
    }

    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignatureDx12> {
        RafxRootSignatureDx12::new(self, root_signature_def)
    }

    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArrayDx12> {
        RafxDescriptorSetArrayDx12::new(self, descriptor_set_array_def)
    }

    pub fn create_graphics_pipeline(
        &self,
        graphics_pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipelineDx12> {
        RafxPipelineDx12::new_graphics_pipeline(self, graphics_pipeline_def)
    }

    pub fn create_compute_pipeline(
        &self,
        compute_pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipelineDx12> {
        RafxPipelineDx12::new_compute_pipeline(self, compute_pipeline_def)
    }

    pub fn create_shader_module(
        &self,
        data: RafxShaderModuleDefDx12,
    ) -> RafxResult<RafxShaderModuleDx12> {
        RafxShaderModuleDx12::new(self, data)
    }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        _resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        Some(candidates[0])

        //unimplemented!();
        /*
            // https://developer.apple.com/metal/Dx12-Feature-Set-Tables.pdf
            use metal_rs::PixelFormatCapabilities;

            let mut required_capabilities = PixelFormatCapabilities::empty();

            // Some formats include color and not write, so I think it's not necessary to have write
            // capability for color attachments
            if resource_type.intersects(RafxResourceType::RENDER_TARGET_COLOR) {
                required_capabilities |= PixelFormatCapabilities::Color;
            }

            // Depth formats don't include write, so presumably it's implied that a depth format can
            // be a depth attachment
            // if resource_type.intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL) {
            //     required_capabilities |= PixelFormatCapabilities::Write;
            // }

            if resource_type.intersects(RafxResourceType::TEXTURE_READ_WRITE) {
                required_capabilities |= PixelFormatCapabilities::Write;
            }

            for &candidate in candidates {
                let capabilities = self
                    .inner
                    .metal_features
                    .pixel_format_capabilities(candidate.into());
                if capabilities.contains(required_capabilities) {
                    return Some(candidate);
                }
            }
        */
        //None
    }

    pub fn find_supported_sample_count(
        &self,
        candidates: &[RafxSampleCount],
    ) -> Option<RafxSampleCount> {
        Some(candidates[0])

        //unimplemented!();
        // for &candidate in candidates {
        //     if self
        //         .inner
        //         .device
        //         .supports_texture_sample_count(candidate.into())
        //     {
        //         return Some(candidate);
        //     }
        // }

        //None
    }
}
