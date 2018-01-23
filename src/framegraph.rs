use winapi::um::d3d12::*;

use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;

use alloc::{
    HeapMemoryAllocator,
    HeapMemoryCacheEntry
};

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

    fn into_resource_state(self) -> D3D12_RESOURCE_STATES {
        let mut out = D3D12_RESOURCE_FLAG_NONE;

        if self.contains(TransitionFlags::RENDER_TARGET) {
            out |= D3D12_RESOURCE_STATE_RENDER_TARGET;
        }

        if self.contains(TransitionFlags::DEPTH_WRITE) {
            out |= D3D12_RESOURCE_STATE_DEPTH_WRITE;
        }

        if self.contains(TransitionFlags::SHADER_RESOURCE) {
            out |= D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
        }

        if self.contains(TransitionFlags::DEPTH_READ) {
            out |= D3D12_RESOURCE_STATE_DEPTH_READ;
        }

        out
    }

    fn into_resource_flags(self) -> D3D12_RESOURCE_FLAGS {
        let mut out = D3D12_RESOURCE_FLAG_NONE;

        if self.contains(TransitionFlags::RENDER_TARGET) {
            out |= D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        }

        if self.contains(TransitionFlags::DEPTH_WRITE) {
            out |= D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL;
        }

        out
    }
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct FrameGraphResource {
    name: &'static str,
    view_id: u32,
    resource_id: u32
}

pub trait ResourceBinding {
    type PhysicalResource;

    fn get_virtual_resource(&self) -> FrameGraphResource;
    fn get_virtual_resources(&self) -> Box<[FrameGraphResource]> {
        Box::new([self.get_virtual_resource()])
    }

    // TODO: TEMP
    fn is_cpu(&self) -> bool;
    fn is_cpus(&self) -> Box<[bool]> {
        Box::new([self.is_cpu()])
    }
}

pub trait IntoTypedResource<T> {
    fn get_virtual_resource(&self) -> FrameGraphResource;
}

macro_rules! physical_resource_bind {
    ($name:ident => CPU) => {
        pub struct $name(FrameGraphResource);

        impl ResourceBinding for $name {
            type PhysicalResource = D3D12_CPU_DESCRIPTOR_HANDLE;

            fn get_virtual_resource(&self) -> FrameGraphResource {
                self.0
            }

            fn is_cpu(&self) -> bool {
                true
            }
        }
    };
    ($name:ident => GPU) => {
        pub struct $name(FrameGraphResource);

        impl ResourceBinding for $name {
            type PhysicalResource = D3D12_GPU_DESCRIPTOR_HANDLE;

            fn get_virtual_resource(&self) -> FrameGraphResource {
                self.0
            }

            fn is_cpu(&self) -> bool {
                false
            }
        }
    };
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

impl ResourceBinding for () {
    type PhysicalResource = ();

    fn get_virtual_resource(&self) -> FrameGraphResource {
        unimplemented!()
    }

    fn is_cpu(&self) -> bool {
        unimplemented!()
    }
}

physical_resource_bind!(RenderTargetResource => CPU);
physical_resource_bind!(DepthStencilResource => CPU);
physical_resource_bind!(ShaderResource => GPU);
physical_resource_bind!(DepthReadResource => CPU);
physical_resource_bind!(DepthWriteResource => CPU);

typed_resource_transition!(RenderTargetResource => ShaderResource);
typed_resource_transition!(DepthReadResource => ShaderResource);
typed_resource_transition!(DepthWriteResource => ShaderResource);
typed_resource_transition!(DepthWriteResource => DepthReadResource);
typed_resource_transition!(DepthReadResource => DepthWriteResource);

#[derive(Derivative)]
#[derivative(Debug)]
struct RenderPass {
    resources: Vec<(u32, TransitionFlags)>,
    #[derivative(Debug="ignore")]
    views: Vec<ResourceView>,
    #[derivative(Debug="ignore")]
    exec: Box<FnMut(*mut ID3D12GraphicsCommandList, &())>,
    param_size: usize,
    params: Vec<(bool, u32, u32)>,
    refcount: u32
}

#[derive(Debug, Copy, Clone)]
struct ResourceTransition {
    resource: u32,
    from: TransitionFlags,
    to: TransitionFlags
}

enum ResourceBarrier {
    Transition(*mut ID3D12Resource, D3D12_RESOURCE_STATES, D3D12_RESOURCE_STATES),
    Alias(*mut ID3D12Resource, *mut ID3D12Resource)
}

impl Into<D3D12_RESOURCE_BARRIER> for ResourceBarrier {
    fn into(self) -> D3D12_RESOURCE_BARRIER {
        match self {
            ResourceBarrier::Transition(resource, from, to) => {
                unsafe {
                    let mut barrier: D3D12_RESOURCE_BARRIER = ::std::mem::zeroed();

                    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
                    barrier.Flags = D3D12_RESOURCE_BARRIER_FLAG_NONE;

                    (*barrier.u.Transition_mut()) = D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: resource,
                        Subresource: 0,
                        StateBefore: from,
                        StateAfter: to,
                    };

                    barrier
                }
            },
            ResourceBarrier::Alias(before, after) => {
                unsafe {
                    let mut barrier: D3D12_RESOURCE_BARRIER = ::std::mem::zeroed();

                    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_ALIASING;
                    barrier.Flags = D3D12_RESOURCE_BARRIER_FLAG_NONE;

                    (*barrier.u.Aliasing_mut()) = D3D12_RESOURCE_ALIASING_BARRIER {
                        pResourceBefore: before,
                        pResourceAfter: after
                    };

                    barrier
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TransientResourceLifetime {
    pub start: u32,
    pub end: u32
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TransientResource {
    refcount: u32,
    resource_id: u32,
    usage: TransitionFlags,
    pub lifetime: TransientResourceLifetime,
    pub size: u64,
    alignment: u64,
    #[derivative(Debug="ignore")]
    pub desc: D3D12_RESOURCE_DESC,
    pub name: &'static str
}

impl ::std::hash::Hash for TransientResource {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self.usage.hash(state);
        self.lifetime.start.hash(state);
        self.lifetime.end.hash(state);
        self.size.hash(state);
    }
}

pub struct FrameGraph {
    device: *mut ID3D12Device,

    // TODO: renderpass contents should borrow from heap allocator (cache)
    renderpasses: Vec<RenderPass>,
    // transitions too? at least aliasing
    renderpass_transitions: Vec<Vec<ResourceTransition>>,
    final_transitions: Vec<Option<TransitionFlags>>,
    resource_aliasing: Vec<bool>,

    resources: Vec<TransientResource>,
    views: Vec<ResourceView>,
    heaps: HeapMemoryAllocator,

    virtual_offset: u32,
    virtual_view: u32
}

impl FrameGraph {
    pub fn new(device: *mut ID3D12Device) -> Self {
        FrameGraph {
            device,
            renderpasses: Vec::new(),
            renderpass_transitions: Vec::new(),
            final_transitions: Vec::new(),
            resource_aliasing: Vec::new(),
            resources: Vec::new(),
            views: Vec::new(),
            heaps: HeapMemoryAllocator::new(device),
            virtual_offset: 0,
            virtual_view: 0,
        }
    }

    pub fn add_pass<T, Init>(&mut self, name: &'static str, init: Init, exec: Box<FnMut(*mut ID3D12GraphicsCommandList, &T::PhysicalResource)>) -> T
        where T: ResourceBinding + Sized /*+ Copy + Clone */,
              Init: FnOnce(&mut FrameGraphBuilder) -> T,
    {
        let mut builder = FrameGraphBuilder::new(self.device, self.virtual_offset, self.virtual_view);

        let output = init(&mut builder);

        self.virtual_offset = builder.counter;
        self.virtual_view = builder.view_counter;

        let device = self.device;

        self.resources.extend(builder.created.into_iter().map(|resource| {
            let (size, alignment) = unsafe {
                let alloc_info = (*device).GetResourceAllocationInfo(0, 1, &resource.desc as *const _);

                (alloc_info.SizeInBytes, alloc_info.Alignment)
            };

            TransientResource {
                refcount: 0,
                resource_id: resource.resource_id,
                usage: resource.flags,
                lifetime: TransientResourceLifetime { start: 0, end: 0 },
                // TODO: temp
                size: size + alignment ,
                alignment: alignment as _,
                desc: resource.desc,
                name: resource.name
            }
        }));

        self.views.extend(builder.views.clone());

        let exec = unsafe { ::std::mem::transmute(exec) };

        self.renderpasses.push((RenderPass {
            resources: builder.resources,
            views: builder.views,
            exec: exec,
            params: output.get_virtual_resources().iter().zip(output.is_cpus().iter()).map(|(r, &b)| (b, r.view_id, r.resource_id)).collect(),
            param_size: output.is_cpus().iter().fold(0usize, |sum, &b| if b { ::std::mem::size_of::<D3D12_CPU_DESCRIPTOR_HANDLE>() } else { ::std::mem::size_of::<D3D12_GPU_DESCRIPTOR_HANDLE>() }),
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
        // TODO: cache all Vec allocations
        //
        let mut current_states = vec![TransitionFlags::empty(); self.resources.len()];
        let mut prev_states = vec![TransitionFlags::empty(); self.resources.len()];
        let mut cache_passes = vec![None; self.resources.len()];
        let mut prev_passes = vec![0usize; self.resources.len()];
        let mut aggregate_state = vec![TransitionFlags::empty(); self.resources.len()];

        self.renderpass_transitions.resize(self.renderpasses.len(), Vec::new());
        self.final_transitions.resize(self.resources.len(), None);
        self.resource_aliasing.clear();
        self.resource_aliasing.resize(self.resources.len(), false);

        for (i, pass) in self.renderpasses.iter().enumerate() {
            for resource in &pass.resources {
                let idx = resource.0 as usize;

                let transition = resource.1;
                let prev_transition = prev_states[idx];

                // TODO: disjoint barriers?
                if let Some(last) = self.final_transitions[idx] {
                    self.renderpass_transitions[i].push(ResourceTransition {
                        resource: resource.0,
                        from: last,
                        to: transition
                    });

                    self.final_transitions[idx] = None;
                }

                aggregate_state[idx].insert(transition);

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

        //println!("{:#?}", self.final_transitions);


        for i in 0..self.resources.len() {
            let prev_state = prev_states[i];
            let current_state = current_states[i];

            self.final_transitions[i] = Some(current_state);
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

        //println!("{:#?}", self.renderpass_transitions);

        for (idx, resource) in self.resources.iter_mut().enumerate() {
            resource.desc.Flags = aggregate_state[idx].into_resource_flags();
        }
    }

    pub fn compile(&mut self) {

        use std::time::Instant;

        let now = Instant::now();
        self.cull();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        //println!("Cull: {}us", sec);

        let now = Instant::now();
        self.find_lifetimes();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        //println!("Lifetimes: {}us", sec);

        let now = Instant::now();
        self.generate_barriers();
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        //println!("GenBarriers: {}us", sec);

        let now = Instant::now();
        let mem = self.heaps.pack_heap(&self.resources, &self.views);
        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000.0);
        //println!("PackHeaps: {}us", sec);
    }

    pub fn exec(&mut self, list: *mut ID3D12GraphicsCommandList) {
        let entry = self.heaps.current();

        //println!("{}", "exec!");
        for (pass, trans) in self.renderpasses.iter_mut().zip(self.renderpass_transitions.iter()) {
            let mut barriers = Vec::new();
            let mut data = vec![0xfeu8; pass.param_size];
            let mut cur = 0;

            unsafe {
                for &(b, id, res_id) in &pass.params {
                    let idx = res_id as usize;
                    if !self.resource_aliasing[idx] {
                        let res = self.heaps.get_placed_resource_ptr(idx);
                        barriers.push(ResourceBarrier::Alias(::std::ptr::null_mut(), res).into());

                        self.resource_aliasing[idx] = true;
                    }

                    let sz = if b {
                        let handle: *const u8 = ::std::mem::transmute(&self.heaps.get_cpu_handle(id as usize));
                        let sz = ::std::mem::size_of::<D3D12_CPU_DESCRIPTOR_HANDLE>();
                        ::std::ptr::copy(handle, data[cur..].as_mut_ptr(), sz);

                        sz
                    } else {
                        let handle: *const u8 = ::std::mem::transmute(&self.heaps.get_gpu_handle(id as u64));
                        let sz = ::std::mem::size_of::<D3D12_GPU_DESCRIPTOR_HANDLE>();
                        ::std::ptr::copy(handle, data[cur..].as_mut_ptr(), sz);

                        sz
                    };

                    cur += sz;
                }
            }

            for transition in trans {
                let res = self.heaps.get_placed_resource_ptr(transition.resource as usize);

                let mut barrier: D3D12_RESOURCE_BARRIER = ResourceBarrier::Transition(
                    res,
                    transition.from.into_resource_state(),
                    transition.to.into_resource_state()
                ).into();

                barriers.push(barrier);

            }

            for barrier in &mut barriers {
                unsafe {
                    if barrier.Type == D3D12_RESOURCE_BARRIER_TYPE_TRANSITION {
                        let barrier = barrier.u.Transition_mut();
                        //println!("Transition({:?}): {:?} => {:?}", barrier.pResource, barrier.StateBefore, barrier.StateAfter);
                    } else if barrier.Type == D3D12_RESOURCE_BARRIER_TYPE_ALIASING {
                        let barrier = barrier.u.Aliasing_mut();
                        //println!("Alias({:?} => {:?})", barrier.pResourceBefore, barrier.pResourceAfter);
                    }
                };
            }

            unsafe { (*list).ResourceBarrier(barriers.len() as u32, barriers.as_ptr()); }

            (pass.exec)(list, unsafe { ::std::mem::transmute(data.as_ptr()) })
        }
    }

    pub fn finish(&mut self) {
        self.renderpasses.clear();
        self.renderpass_transitions.clear();
        self.resources.clear();
        self.views.clear();
        self.virtual_offset = 0;
        self.virtual_view = 0;
    }
}


// created: Vec<(&'static str, u32, TransitionFlags, D3D12_RESOURCE_DESC)>,
//#[derive(Debug)]
pub struct PlacedResource {
    name: &'static str,
    resource_id: u32,
    flags: TransitionFlags,
    desc: D3D12_RESOURCE_DESC,
}

#[derive(Clone)]
pub enum ResourceViewDesc {
    RenderTarget(D3D12_RENDER_TARGET_VIEW_DESC),
    ShaderResource(D3D12_SHADER_RESOURCE_VIEW_DESC),
}

//#[derive(Debug)]
#[derive(Clone)]
pub struct ResourceView {
    pub resource_id: u32,
    pub view_id: u32,
    pub desc: ResourceViewDesc
}

//#[derive(Debug)]
pub struct FrameGraphBuilder {
    device: *mut ID3D12Device,
    created: Vec<PlacedResource>,
    resources: Vec<(u32, TransitionFlags)>,
    views: Vec<ResourceView>,
    counter: u32,
    view_counter: u32,
}

impl FrameGraphBuilder {
    fn new(device: *mut ID3D12Device, offset: u32, view_offset: u32) -> Self {
        FrameGraphBuilder {
            device: device,
            created: Vec::new(),
            resources: Vec::new(),
            views: Vec::new(),
            counter: offset,
            view_counter: view_offset
        }
    }

    pub fn create_render_target(&mut self, name: &'static str, desc: RenderTargetDesc) -> RenderTargetResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource {
            name: name,
            view_id: self.view_counter,
            resource_id: virtual_id
        };

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

        let mut view_desc: D3D12_RENDER_TARGET_VIEW_DESC = unsafe { ::std::mem::zeroed() };
        view_desc.Format = desc.format.into();
        view_desc.ViewDimension = D3D12_RTV_DIMENSION_TEXTURE2D;
        unsafe {
            (*view_desc.u.Texture2D_mut()) = D3D12_TEX2D_RTV {
                MipSlice: 0,
                PlaneSlice: 0
            };
        }

        self.views.push(ResourceView {
            resource_id: virtual_id,
            view_id: self.view_counter,
            desc: ResourceViewDesc::RenderTarget(view_desc)
        });

        self.view_counter += 1;

        self.created.push(PlacedResource {
            resource_id: virtual_id,
            flags: TransitionFlags::RENDER_TARGET,
            desc: resource_desc,
            name: name
        });
        self.write(res, TransitionFlags::RENDER_TARGET);

        RenderTargetResource(res)
    }

    pub fn create_depth(&mut self, name: &'static str, desc: DepthDesc) -> DepthWriteResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource {
            name: name,
            view_id: self.view_counter,
            resource_id: virtual_id
        };

        self.view_counter += 1;

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

        self.created.push(PlacedResource {
            resource_id: virtual_id,
            flags: TransitionFlags::DEPTH_WRITE,
            desc: resource_desc,
            name: name
        });
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
        self.resources.push((resource.resource_id, transition));
        resource
    }

    fn write(&mut self, resource: FrameGraphResource, transition: TransitionFlags) -> FrameGraphResource {
        self.resources.push((resource.resource_id, transition));
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
