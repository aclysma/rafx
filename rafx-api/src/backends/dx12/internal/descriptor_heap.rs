use super::d3d12;
use crate::RafxResult;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use windows::Win32::Graphics::Direct3D12::ID3D12DescriptorHeap;

// https://graphics.stanford.edu/~seander/bithacks.html#Round   UpPowerOf2
fn next_power_of_two(mut v: u32) -> u32 {
    v = v.saturating_sub(1);
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v += 1;
    return v;
}
/*
struct NonShaderVisibleDescriptorHeap {
    non_shader_visible_heap: d3d12::ID3D12DescriptorHeap,
    start_cpu_handle: d3d12::D3D12_CPU_DESCRIPTOR_HANDLE,
}

impl NonShaderVisibleDescriptorHeap {
    pub fn new(
        device: &d3d12::ID3D12Device,
        heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
        descriptor_count: u32,
    ) -> RafxResult<Self> {
        let heap_desc = d3d12::D3D12_DESCRIPTOR_HEAP_DESC {
            Type: heap_type,
            NumDescriptors: descriptor_count,
            Flags: d3d12::D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0
        };

        println!("{:?}", heap_desc);

        unsafe {
            let non_shader_visible_heap: d3d12::ID3D12DescriptorHeap = device.CreateDescriptorHeap(&heap_desc)?;
            let start_cpu_handle = non_shader_visible_heap.GetCPUDescriptorHandleForHeapStart();
            println!("start_cpu_handle {}", start_cpu_handle.ptr);

            Ok(NonShaderVisibleDescriptorHeap {
                non_shader_visible_heap,
                start_cpu_handle,
            })
        }
    }
}

struct ShaderVisibleDescriptorHeap {
    shader_visible_heap: d3d12::ID3D12DescriptorHeap,
    start_cpu_handle: d3d12::D3D12_CPU_DESCRIPTOR_HANDLE,
    start_gpu_handle: d3d12::D3D12_GPU_DESCRIPTOR_HANDLE,
}

impl ShaderVisibleDescriptorHeap {
    pub fn new(
        device: &d3d12::ID3D12Device,
        heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
        descriptor_count: u32,
    ) -> RafxResult<Self> {
        let heap_desc = d3d12::D3D12_DESCRIPTOR_HEAP_DESC {
            Type: heap_type,
            NumDescriptors: descriptor_count,
            Flags: d3d12::D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0
        };

        println!("{:?}", heap_desc);

        unsafe {
            let shader_visible_heap: d3d12::ID3D12DescriptorHeap = device.CreateDescriptorHeap(&heap_desc)?;
            let start_cpu_handle = shader_visible_heap.GetCPUDescriptorHandleForHeapStart();
            println!("start_cpu_handle {}", start_cpu_handle.ptr);
            let start_gpu_handle = shader_visible_heap.GetGPUDescriptorHandleForHeapStart();
            println!("start_gpu_handle {}", start_gpu_handle.ptr);
            Ok(ShaderVisibleDescriptorHeap {
                shader_visible_heap,
                start_cpu_handle,
                start_gpu_handle,
            })
        }
    }
}
*/

struct HeapWithHandles {
    heap: ID3D12DescriptorHeap,
    cpu_first_handle: d3d12::D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_first_handle: Option<d3d12::D3D12_GPU_DESCRIPTOR_HANDLE>,
}

fn create_heap(
    device: &d3d12::ID3D12Device,
    heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
    descriptor_count: u32,
    shader_visible: bool,
) -> RafxResult<HeapWithHandles> {
    let mut heap_desc = d3d12::D3D12_DESCRIPTOR_HEAP_DESC {
        Type: heap_type,
        NumDescriptors: descriptor_count,
        Flags: d3d12::D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
        NodeMask: 0,
    };

    if shader_visible {
        heap_desc.Flags |= d3d12::D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    }

    println!("{:?}", heap_desc);

    let heap: ID3D12DescriptorHeap;
    let cpu_first_handle;
    let gpu_first_handle;

    unsafe {
        heap = device.CreateDescriptorHeap(&heap_desc)?;
        cpu_first_handle = heap.GetCPUDescriptorHandleForHeapStart();
        println!("start_cpu_handle {}", cpu_first_handle.ptr);

        gpu_first_handle = if shader_visible {
            Some(heap.GetGPUDescriptorHandleForHeapStart())
        } else {
            None
        };
    }

    Ok(HeapWithHandles {
        heap,
        cpu_first_handle,
        gpu_first_handle,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Dx12DescriptorId(pub u32);

impl Dx12DescriptorId {
    pub fn add_offset(
        self,
        offset: u32,
    ) -> Dx12DescriptorId {
        Dx12DescriptorId(self.0 + offset)
    }
}

pub struct Dx12DescriptorHeapInner {
    heap: d3d12::ID3D12DescriptorHeap,
    cpu_first_handle: d3d12::D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_first_handle: Option<d3d12::D3D12_GPU_DESCRIPTOR_HANDLE>,
    descriptor_count: u32,
    heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
    allocated_descriptor_count: u32,
    //TODO: bitfield
    allocated_descriptors: Vec<bool>,
}

impl Dx12DescriptorHeapInner {
    pub fn new(
        device: &d3d12::ID3D12Device,
        heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
        descriptor_count: u32,
        shader_visible: bool,
    ) -> RafxResult<Self> {
        let heap = create_heap(device, heap_type, descriptor_count, shader_visible)?;

        let allocated_descriptors = vec![false; descriptor_count as usize];

        Ok(Dx12DescriptorHeapInner {
            heap: heap.heap,
            cpu_first_handle: heap.cpu_first_handle,
            gpu_first_handle: heap.gpu_first_handle,

            //start_cpu_handle: non_shader_visible_heap.start_cpu_handle,
            //shader_visible_heap,
            descriptor_count,
            heap_type,
            allocated_descriptor_count: 0,
            allocated_descriptors,
        })
    }

    fn grow(
        &mut self,
        device: &d3d12::ID3D12Device,
        minimum_required_descriptors: u32,
    ) -> RafxResult<()> {
        let old_size = self.descriptor_count;
        let new_size = next_power_of_two(old_size + minimum_required_descriptors);

        println!("GROWING HEAP {} -> {}", old_size, new_size);
        let shader_visible = self.gpu_first_handle.is_some();

        // Copy into the new heap
        //let new_heap = NonShaderVisibleDescriptorHeap::allocate(device, self.heap_type, new_size)?;
        let new_heap = create_heap(device, self.heap_type, new_size, shader_visible)?;
        unsafe {
            device.CopyDescriptorsSimple(
                old_size,
                new_heap.cpu_first_handle,
                self.cpu_first_handle,
                self.heap_type,
            );
        }

        // Drops the old smaller heap
        self.heap = new_heap.heap;
        self.cpu_first_handle = new_heap.cpu_first_handle;
        self.gpu_first_handle = new_heap.gpu_first_handle;
        self.descriptor_count = new_size;
        self.allocated_descriptors.resize(new_size as usize, false);

        // if let Some(gpu_first_handle) = new_heap.gpu_first_handle {
        //     // // Copy into the new shader-visible heap
        //     // //let new_shader_visible_heap = ShaderVisibleDescriptorHeap::allocate(device, self.heap_type, new_size)?;
        //     // //let new_shader_visible_heap = create_heap(device, self.heap_type, new_size, shader_visible);
        //     // unsafe {
        //     //     device.CopyDescriptorsSimple(
        //     //         old_size,
        //     //         new_shader_visible_heap.start_cpu_handle,
        //     //         shader_visible_heap.start_cpu_handle,
        //     //         self.heap_type
        //     //     );
        //     // }
        //     //
        //     // // Drop the old smaller shader-visible heap
        //     // shader_visible_heap.shader_visible_heap = new_shader_visible_heap.shader_visible_heap;
        //     // shader_visible_heap.start_cpu_handle = new_shader_visible_heap.start_cpu_handle;
        //     // shader_visible_heap.start_gpu_handle = new_shader_visible_heap.start_gpu_handle;
        //     self.gpu_first_handle = gpu_first_handle;
        // }

        Ok(())
    }

    pub fn allocate(
        &mut self,
        device: &d3d12::ID3D12Device,
        count: u32,
    ) -> RafxResult<Dx12DescriptorId> {
        assert!(count > 0);

        //let mut free_descriptors = 0;
        //let mut first_free_descriptor = 0;
        let mut free_count = 0;
        let mut free_range_begin = 0;
        for i in 0..self.descriptor_count {
            if !self.allocated_descriptors[i as usize] {
                free_count += 1;
            } else {
                free_count = 0;
                free_range_begin = i + 1;
            }

            if free_count >= count {
                break;
            }
        }

        if free_count < count {
            self.grow(device, count)?;
        }

        for i in free_range_begin..(free_range_begin + count) {
            self.allocated_descriptors[i as usize] = true;
        }

        self.allocated_descriptor_count += count;

        Ok(Dx12DescriptorId(free_range_begin))
    }

    pub fn free(
        &mut self,
        first_descriptor: Dx12DescriptorId,
        count: u32,
    ) {
        assert!(count > 0);

        for i in first_descriptor.0..(first_descriptor.0 + count) {
            assert!(self.allocated_descriptors[i as usize]);
            self.allocated_descriptors[i as usize] = false;
        }

        self.allocated_descriptor_count -= count;
    }
}

pub struct Dx12DescriptorHeap {
    inner: Arc<Mutex<Dx12DescriptorHeapInner>>,
    // Cache this immutable data outside the mutex so that we can access without locking
    //dx12_heap: ID3D12DescriptorHeap,
    heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
    stride: u32,
    cpu_first_handle: AtomicUsize,
    // If 0xFFFFFFFFFFFFFFFF, it's invalid
    gpu_first_handle: AtomicU64,
}

impl Dx12DescriptorHeap {
    pub fn new(
        device: &d3d12::ID3D12Device,
        heap_type: d3d12::D3D12_DESCRIPTOR_HEAP_TYPE,
        descriptor_count: u32,
        shader_visible: bool,
    ) -> RafxResult<Self> {
        let heap =
            Dx12DescriptorHeapInner::new(device, heap_type, descriptor_count, shader_visible)?;
        let cpu_first_handle = AtomicUsize::new(heap.cpu_first_handle.ptr);
        let gpu_first_handle = if let Some(gpu_first_handle) = &heap.gpu_first_handle {
            AtomicU64::new(gpu_first_handle.ptr)
        } else {
            AtomicU64::new(0xFFFFFFFFFFFFFFFF)
        };

        let stride = unsafe { device.GetDescriptorHandleIncrementSize(heap_type) };
        //let dx12_heap = heap.heap.clone();
        Ok(Dx12DescriptorHeap {
            inner: Arc::new(Mutex::new(heap)),
            //dx12_heap,
            heap_type,
            stride,
            cpu_first_handle,
            gpu_first_handle,
        })
    }

    // pub fn dx12_heap(&self) -> &d3d12::ID3D12DescriptorHeap {
    //     &self.dx12_heap
    // }

    pub fn heap_type(&self) -> d3d12::D3D12_DESCRIPTOR_HEAP_TYPE {
        self.heap_type
    }

    pub fn dx12_heap(&self) -> d3d12::ID3D12DescriptorHeap {
        let inner = self.inner.lock().unwrap();
        inner.heap.clone()
    }

    // pub fn gpu_visible_heap(&self) -> Option<d3d12::ID3D12DescriptorHeap> {
    //     let inner = self.inner.lock().unwrap();
    //     inner.shader_visible_heap.as_ref().map(|x| x.shader_visible_heap.clone())
    // }

    pub fn allocate(
        &self,
        device: &d3d12::ID3D12Device,
        count: u32,
    ) -> RafxResult<Dx12DescriptorId> {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.allocate(device, count);

        // We may have grown the heap. In case we did, refresh the cached first handles as they may
        // be stale now
        self.cpu_first_handle
            .store(inner.cpu_first_handle.ptr, Ordering::Relaxed);
        if let Some(gpu_first_handle) = inner.gpu_first_handle {
            self.gpu_first_handle
                .store(gpu_first_handle.ptr, Ordering::Relaxed);
        }

        id
    }

    pub fn free(
        &self,
        first_descriptor: Dx12DescriptorId,
        count: u32,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.free(first_descriptor, count)
    }

    pub fn id_to_cpu_handle(
        &self,
        id: Dx12DescriptorId,
    ) -> d3d12::D3D12_CPU_DESCRIPTOR_HANDLE {
        let start_cpu_handle = self.cpu_first_handle.load(Ordering::Relaxed);
        d3d12::D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: start_cpu_handle + (id.0 * self.stride) as usize,
        }
    }

    pub fn id_to_gpu_handle(
        &self,
        id: Dx12DescriptorId,
    ) -> d3d12::D3D12_GPU_DESCRIPTOR_HANDLE {
        let start_gpu_handle = self.gpu_first_handle.load(Ordering::Relaxed);
        assert_ne!(start_gpu_handle, 0xFFFFFFFFFFFFFFFF);
        d3d12::D3D12_GPU_DESCRIPTOR_HANDLE {
            ptr: start_gpu_handle + (id.0 * self.stride) as u64,
        }
    }

    // pub fn id_to_shader_visible_cpu_handle(&self, id: Dx12DescriptorId) -> d3d12::D3D12_CPU_DESCRIPTOR_HANDLE {
    //     let start_cpu_handle = self.start_shader_visible_start_cpu_handle.load(Ordering::Relaxed);
    //     assert_ne!(start_cpu_handle, 0xFFFFFFFFFFFFFFFF);
    //     d3d12::D3D12_CPU_DESCRIPTOR_HANDLE {
    //         ptr: start_cpu_handle + (id.0 * self.stride) as usize
    //     }
    // }
    //
    // pub fn id_to_shader_visible_gpu_handle(&self, id: Dx12DescriptorId) -> d3d12::D3D12_GPU_DESCRIPTOR_HANDLE {
    //     let start_gpu_handle = self.start_shader_visible_start_gpu_handle.load(Ordering::Relaxed);
    //     assert_ne!(start_gpu_handle, 0xFFFFFFFFFFFFFFFF);
    //     d3d12::D3D12_GPU_DESCRIPTOR_HANDLE {
    //         ptr: start_gpu_handle + (id.0 * self.stride) as u64
    //     }
    // }
}

pub struct Dx12DescriptorHeapSet {
    // Non-shader-visible heaps. We use these for resources that don't need to be shader visible,
    // or for creating resource views that will be copied to shader-visible heaps later

    // CBVs, SRVs, UAVs
    pub cbv_srv_uav_heap: Dx12DescriptorHeap,
    pub sampler_heap: Dx12DescriptorHeap,
    // render target view
    pub rtv_heap: Dx12DescriptorHeap,
    // depth stencil view
    pub dsv_heap: Dx12DescriptorHeap,

    // Shader visible heaps
    pub gpu_cbv_srv_uav_heap: Dx12DescriptorHeap,
    pub gpu_sampler_heap: Dx12DescriptorHeap,
}

impl Dx12DescriptorHeapSet {
    pub fn new(device: &d3d12::ID3D12Device) -> RafxResult<Self> {
        // D3D12_MAX_SHADER_VISIBLE_DESCRIPTOR_HEAP_SIZE_TIER_1 limits this to 1M
        let cbv_srv_uav_heap = Dx12DescriptorHeap::new(
            device,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            256 * 1024,
            false,
        )?;
        // D3D12_MAX_SHADER_VISIBLE_SAMPLER_HEAP_SIZE limits this to 2048
        let sampler_heap = Dx12DescriptorHeap::new(
            device,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
            2048,
            false,
        )?;
        let rtv_heap =
            Dx12DescriptorHeap::new(device, d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_RTV, 512, false)?;
        let dsv_heap =
            Dx12DescriptorHeap::new(device, d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_DSV, 512, false)?;

        let gpu_cbv_srv_uav_heap = Dx12DescriptorHeap::new(
            device,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            256 * 1024,
            true,
        )?;
        let gpu_sampler_heap = Dx12DescriptorHeap::new(
            device,
            d3d12::D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
            2048,
            true,
        )?;

        Ok(Dx12DescriptorHeapSet {
            cbv_srv_uav_heap,
            sampler_heap,
            rtv_heap,
            dsv_heap,
            gpu_cbv_srv_uav_heap,
            gpu_sampler_heap,
        })
    }
}
