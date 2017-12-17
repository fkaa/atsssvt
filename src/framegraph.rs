use winapi::um::d3d12::*;
use winapi::um::d3d12sdklayers::*;
use winapi::um::d3dcommon::*;

use winapi::shared::winerror::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::dxgi::*;
//use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgi1_3::*;
use winapi::shared::dxgi1_4::*;

use winapi::Interface;

use std::ptr;
use std::mem;

bitflags! {
    struct TransitionFlags: u32 {
        const RENDER_TARGET = 0x1;
        const SHADER_RESOURCE = 0x2;
        const DEPTH_WRITE = 0x4;
        const DEPTH_READ = 0x8;
    }
}

impl TransitionFlags {
    fn has_read(self) -> bool {
        self.intersects(TransitionFlags::SHADER_RESOURCE | TransitionFlags::DEPTH_READ)
    }

    fn has_write(self) -> bool {
        self.intersects(TransitionFlags::RENDER_TARGET | TransitionFlags::DEPTH_WRITE)
    }
}

// TODO: restructure according to dice slides:
//         [x] renderpass holds array of used resources (and direction?, R/W)
//         [x] renderpasses is a flat array in framegraph
//         [x] resources also flat array, referenced by renderpasses
//         [x] compile time:
//           [x] iterate over renderpasses:
//             [x] compute resource ref count
//             [x] compute first and last user (renderpass)
//             [x] compute barriers (between renderpasses ?)
//         [/] culling:
//           [x] pass.rc += 1 for resource writes
//           [x] resource.rc += for resource reads
//           [x] push resources with rc == 0 to stack
//             [/] while !empty():
//               [?] pop, resource.producer.rc--
//               [?] if resource.producer.rc == 0:
//                 [?] resource.producer.reads.rc--
//                 [?] if resource.producer.reads.rc == 0
//                   [?] push to stack
//
// TODO: prepare for DX12 implementation:
//         [ ] move device creation into main
//         [ ] single large heap
//         [ ] how to deal with allocation "scheduling"?
//           * NOTE: large-ish heap with smaller "overflow" heaps?
//           * NOTE: mostly persistent, allow fragmentation?
//           * NOTE: manual flag on adding pass?
//         [/] get physical size of resources
//           [/] just need to supply the correct D3D12 resource desc
//         [ ] fix proper creation descriptions 
//          
//
//         NOTE: how to do alloc:
//                * find all resource size bins
//                  * find candidate by looking at bins with size >= res.size
//                  * if lifetimes dont overlap, alias, with some heuristic (
//                    more bytes saves by aliasing? maybe some rect-fitting
//                    alg that works)
//
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct FrameGraphResource(&'static str, u32);

pub trait ResourceBinding {
    type PhysicalResource;

    fn get_virtual_resource(&self) -> FrameGraphResource;
}

pub trait IntoTypedResource<T> {
    fn get_virtual_resource(&self) -> FrameGraphResource;
}

macro_rules! physical_resource_bind {
    ($name:ident => $physical:ty) => {
        pub struct $name(FrameGraphResource);

        impl ResourceBinding for $name {
            type PhysicalResource = $physical;

            fn get_virtual_resource(&self) -> FrameGraphResource {
                self.0
            }
        }
    }
}

macro_rules! typed_resource_transition {
    ($type:ty => $target:ty) => {
        impl IntoTypedResource<$target> for $type {
            fn get_virtual_resource(&self) -> FrameGraphResource {
                self.0
            }
        }
    }
}

physical_resource_bind!(RenderTargetResource => ());
physical_resource_bind!(ShaderResource => ());
physical_resource_bind!(DepthStencilResource => ());
physical_resource_bind!(DepthReadResource => ());
physical_resource_bind!(DepthWriteResource => ());

typed_resource_transition!(RenderTargetResource => ShaderResource);
typed_resource_transition!(DepthReadResource => ShaderResource);
typed_resource_transition!(DepthWriteResource => ShaderResource);
typed_resource_transition!(DepthWriteResource => DepthReadResource);
typed_resource_transition!(DepthReadResource => DepthWriteResource);

#[derive(Debug)]
struct RenderPass {
    resources: Vec<(u32, TransitionFlags)>,
    refcount: u32
}

#[derive(Debug, Copy, Clone)]
struct ResourceTransition {
    resource: u32,
    from: TransitionFlags,
    to: TransitionFlags
}

#[derive(Debug, Copy, Clone)]
pub struct TransientResourceLifetime {
    pub start: u32,
    pub end: u32
}

#[derive(Debug)]
pub struct TransientResource {
    refcount: u32,
    usage: TransitionFlags,
    lifetime: TransientResourceLifetime,
    size: u64,
    alignment: u64,
    name: &'static str
}

pub struct FrameGraph {
    device: *mut ID3D12Device,

    renderpasses: Vec<RenderPass>,
    renderpass_transitions: Vec<Vec<ResourceTransition>>,

// (u32, TransitionFlags, Option<usize>, Option<usize>, &'static str)
    resources: Vec<TransientResource>,

    virtual_offset: u32
}

impl FrameGraph {
    pub fn new() -> Self {
        unsafe {
            let mut debug_controller: *mut ID3D12Debug = ptr::null_mut();
            if SUCCEEDED(D3D12GetDebugInterface(&ID3D12Debug::uuidof(), mem::transmute(&mut debug_controller))) {
                (*debug_controller).EnableDebugLayer();
            }

            let mut factory: *mut IDXGIFactory4 = ptr::null_mut();
            if !SUCCEEDED(CreateDXGIFactory2(DXGI_CREATE_FACTORY_DEBUG, &IDXGIFactory1::uuidof(), mem::transmute(&mut factory))) {
                panic!();
            }

            let mut adapter: *mut IDXGIAdapter1 = ptr::null_mut();
            let mut idx = 0;
            while (*factory).EnumAdapters1(idx, &mut adapter as _) != DXGI_ERROR_NOT_FOUND {
                let mut desc: DXGI_ADAPTER_DESC1 = mem::uninitialized();
                if !SUCCEEDED((*adapter).GetDesc1(&mut desc)) {
                    idx = idx + 1;
                    continue;
                }

                if SUCCEEDED(D3D12CreateDevice(
                    mem::transmute(adapter),

                    D3D_FEATURE_LEVEL_11_0,
                    &ID3D12Device::uuidof(),
                    ptr::null_mut()))
                {
                    break;
                }

                idx = idx + 1;
            }

            let mut device: *mut ID3D12Device = ptr::null_mut();
            D3D12CreateDevice(
                mem::transmute(adapter.as_mut()),
                D3D_FEATURE_LEVEL_11_0,
                &ID3D12Device::uuidof(),
                mem::transmute(&mut device));

            FrameGraph {
                device,
                renderpasses: Vec::new(),
                renderpass_transitions: Vec::new(),
                resources: Vec::new(),
                virtual_offset: 0,
            }
        }
    }

    pub fn add_pass<T, Init, Exec>(&mut self, name: &'static str, init: Init, exec: Exec) -> T
        where T: Sized /*+ Copy + Clone */,
              Init: FnOnce(&mut FrameGraphBuilder) -> T,
              Exec: FnMut(T)
    {
        let mut builder = FrameGraphBuilder::new(self.device, self.virtual_offset);

        let output = init(&mut builder);

        self.virtual_offset = builder.counter;

        let device = self.device;

        self.resources.extend(builder.created.into_iter().map(|(n,a,b,desc)| {
            let (size, alignment) = unsafe {
                let alloc_info = (*device).GetResourceAllocationInfo(0, 1, &desc as *const _);

                (alloc_info.SizeInBytes, alloc_info.Alignment)
            };

            TransientResource {
                refcount: 0,
                usage: b,
                lifetime: TransientResourceLifetime { start: 0, end: 0 },
                size: size as _,
                alignment: alignment as _,
                name: n
            }
        }));

        self.renderpasses.push((RenderPass {
            resources: builder.resources,
            refcount: 0
        }));

        output
    }

    pub fn cull(&mut self) {
        // TODO: move to builder-phase?
        for pass in &mut self.renderpasses {
            for resource in &pass.resources {
                if resource.1.has_read() {
                    self.resources[resource.0 as usize].refcount += 1;
                } else if resource.1.has_write() {
                    pass.refcount += 1;
                }
            }
        }

        use ::std::collections::vec_deque::VecDeque;

        // push all resources that are never read to stack
        let mut unused = VecDeque::new();
        for (idx, resource) in self.resources.iter().enumerate() {
            if resource.refcount == 0 {
                unused.push_back(idx);
            }
        }

        let mut prune_list = Vec::new();

        loop {
            // while the stack is not empty,
            if let Some(resource) = unused.pop_front() {

                // decrease the producers rc (if a pass's outputs are never
                // read and has no side-effects then it can be culled)
                let mut producer = &mut self.renderpasses[resource];
                producer.refcount -= 1;
               
                // to cull, we push all of that pass's reads onto the unused
                // stack (they are now suspects!)
                //
                // repeat until stack is empty
                if producer.refcount == 0 {
                    prune_list.push(self.resources[resource].lifetime.start);

                    for resource in &producer.resources {
                        if resource.1.has_read() {
                            self.resources[resource.0 as usize].refcount -= 1;
                            if self.resources[resource.0 as usize].refcount == 0 {
                                unused.push_back(resource.0 as usize);
                            }
                        }
                    }
                } 
            } else {
                break;
            }
        }

        // TODO: sort prune list so we delete in right order
        // remove our culled passes
        for i in prune_list {
            self.renderpasses.remove(i as usize);
        }
    }

    fn find_lifetimes(&mut self) {
        // find first and last usage of a resource
        for (idx, mut resource) in self.resources.iter_mut().enumerate() {
            let first_use = self.renderpasses.iter().position(|pass| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());
            let last_use = self.renderpasses.iter().rposition(|pass| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());

            resource.lifetime = TransientResourceLifetime {
                start: first_use.unwrap() as u32,
                end: last_use.unwrap() as u32
            };
        }
    }

    fn generate_barriers(&mut self) {
        // generate all transition barriers
        //
        // TODO: maybe cull passes before?
        //
        let mut current_states = vec![TransitionFlags::empty(); self.resources.len()];
        let mut prev_states = vec![TransitionFlags::empty(); self.resources.len()];
        let mut cache_passes = vec![None; self.resources.len()];
        let mut prev_passes = vec![0usize; self.resources.len()];

        self.renderpass_transitions.resize(self.renderpasses.len(), Vec::new());

        for (i, pass) in self.renderpasses.iter().enumerate() {
            for resource in &pass.resources {
                let idx = resource.0 as usize;

                let transition = resource.1;
                let prev_transition = prev_states[idx];

                if transition.has_write() {
                    if current_states[idx].has_read() {
                        if let Some(pass_idx) = cache_passes[idx] {
                            self.renderpass_transitions[pass_idx as usize].push(ResourceTransition {
                                resource: resource.0,
                                from: prev_states[idx],
                                to: current_states[idx]
                            });

                            prev_states[idx] = current_states[idx];
                            cache_passes[idx] = None;
                        }
                    }

                    if let Some(pass_idx) = cache_passes[idx] {
                        self.renderpass_transitions[pass_idx as usize].push(ResourceTransition {
                            resource: resource.0,
                            from: prev_states[idx],
                            to: current_states[idx]
                        });

                        cache_passes[idx] = Some(prev_passes[idx]);
                        prev_states[idx] = current_states[idx];
                        current_states[idx] = transition;
                    } else {
                        if prev_transition.is_empty() {
                            prev_states[idx] = transition;
                        }

                        current_states[idx] = transition;
                        cache_passes[idx] = Some(i);
                    }
                } else {
                    if current_states[idx].has_write() {
                        prev_states[idx] = current_states[idx];
                        cache_passes[idx] = Some(i);
                        current_states[idx] = transition;
                    }

                    current_states[idx].insert(transition);
                }

                prev_passes[idx] = i;
            }
        }


        for i in 0..self.resources.len() {
            let prev_state = prev_states[i];
            let current_state = current_states[i];

            if prev_state != current_state {
                if let Some(pass_idx) = cache_passes[i] {
                    self.renderpass_transitions[pass_idx].push(ResourceTransition {
                        resource: i as u32,
                        from: prev_state,
                        to: current_state
                    });
                }
            }
        }
    }

    pub fn compile(&mut self) {

        use std::time::Instant;

        let now = Instant::now();
        self.cull();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        println!("Cull: {}us", sec);

        let now = Instant::now();
        self.find_lifetimes();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        println!("Lifetimes: {}us", sec);

        let now = Instant::now();
        self.generate_barriers();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        println!("GenBarriers: {}us", sec);

        let now = Instant::now();
        use alloc::HeapMemoryAllocator;
        let bins = HeapMemoryAllocator::with_resources(self.resources.iter().map(|r| (r.size, r.lifetime)).collect::<Vec<_>>());
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        println!("PackHeaps: {}us", sec);

        //println!("Resources: {:#?}", self.resources);
        //println!("Renderpasses: {:#?}", self.renderpasses);
        println!("Bins: {:#?}", bins);
    }

    pub fn dump(&mut self) {
    }
}

// TODO: each placed resource needs to be created with all usages known
//       ahead of time (RT, DT) and also be able to transition into proper
//       states (SRV, RTV, DSV)
//
// TODO: heap only requires `HEAP_ALLOW_RT_DS`?
//#[derive(Debug)]
pub struct FrameGraphBuilder {
    device: *mut ID3D12Device,
    created: Vec<(&'static str, u32, TransitionFlags, D3D12_RESOURCE_DESC)>,
    resources: Vec<(u32, TransitionFlags)>,
    counter: u32,
}

impl FrameGraphBuilder {
    fn new(device: *mut ID3D12Device, offset: u32) -> Self {
        FrameGraphBuilder {
            device: device,
            created: Vec::new(),
            resources: Vec::new(),
            counter: offset
        }
    }

    pub fn create_render_target(&mut self, name: &'static str, desc: RenderTargetDesc) -> RenderTargetResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);

        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: 1280,
            Height: 720,
            DepthOrArraySize: 1,
            MipLevels: desc.mip_levels as u16,
            Format: desc.format.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 1
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };

        self.created.push((name, virtual_id, TransitionFlags::RENDER_TARGET, resource_desc));
        self.write(res, TransitionFlags::RENDER_TARGET);

        RenderTargetResource(res)
    }

    pub fn create_depth(&mut self, name: &'static str, desc: DepthDesc) -> DepthWriteResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);

        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: 1280,
            Height: 720,
            DepthOrArraySize: 1,
            MipLevels: 1,// desc.mip_levels as u16,
            Format: desc.format.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 1
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };

        self.created.push((name, virtual_id, TransitionFlags::DEPTH_WRITE, resource_desc));
        self.write(res, TransitionFlags::DEPTH_WRITE);

        DepthWriteResource(res)
    }

    pub fn read_srv<T: IntoTypedResource<ShaderResource>>(&mut self, resource: &T) -> ShaderResource {
        ShaderResource(self.read(resource.get_virtual_resource(), TransitionFlags::SHADER_RESOURCE))
    }

    pub fn read_depth<T: IntoTypedResource<DepthReadResource>>(&mut self, resource: &T) -> DepthReadResource {
        DepthReadResource(self.read(resource.get_virtual_resource(), TransitionFlags::DEPTH_READ))
    }

    pub fn write_depth<T: IntoTypedResource<DepthWriteResource>>(&mut self, resource: T) -> DepthWriteResource {
        DepthWriteResource(self.write(resource.get_virtual_resource(), TransitionFlags::DEPTH_WRITE))
    }

    fn read(&mut self, resource: FrameGraphResource, transition: TransitionFlags) -> FrameGraphResource {
        self.resources.push((resource.1, transition));
        resource
    }

    fn write(&mut self, resource: FrameGraphResource, transition: TransitionFlags) -> FrameGraphResource {
        self.resources.push((resource.1, transition));
        resource
    }
}




#[derive(Debug, Copy, Clone)]
pub enum TextureSize {
    Full,
    Half,
    Explicit(u32, u32)
}

#[derive(Debug, Copy, Clone)]
pub enum InitialResourceState {
    Clear,
    DontCare
}

#[derive(Debug, Copy, Clone)]
pub enum ResourceState {
    Clear,
    DontCare
}

#[derive(Debug, Copy, Clone)]
pub enum DepthFormat {
    D32,
    D24
}

#[derive(Debug, Copy, Clone)]
pub struct DepthDesc {
    pub format: DepthFormat,
    pub size: TextureSize,
    pub state: InitialResourceState,
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFormat {
    RGBA8,
    R8,
}

impl From<DepthFormat> for DXGI_FORMAT {
    fn from(f: DepthFormat) -> DXGI_FORMAT {

        match f {
            _ => DXGI_FORMAT_D32_FLOAT
            // DepthFormat::D32 => DXGI_FORMAT_D32_FLOAT,
            // DepthFormat::D24 => DXGI_FORMAT_D24_FLOAT,
        }
    }
}

impl From<TextureFormat> for DXGI_FORMAT {
    fn from(f: TextureFormat) -> DXGI_FORMAT {

        match f {
            TextureFormat::RGBA8 => DXGI_FORMAT_R8G8B8A8_UNORM,
            TextureFormat::R8 => DXGI_FORMAT_R8_UNORM
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RenderTargetDesc {
    pub format: TextureFormat,
    pub size: TextureSize,
    pub mip_levels: u32,
    pub state: InitialResourceState,
}
