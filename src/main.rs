#![allow(dead_code)]
#![allow(unused_variables)]

extern crate winapi;
extern crate term;

#[macro_use]
extern crate bitflags;

/*use winapi::um::d3d12::{
    D3D12_RESOURCE_STATES,
    D3D12_RESOURCE_STATE_RENDER_TARGET,
    D3D12_RESOURCE_STATE_DEPTH_WRITE,
    D3D12_RESOURCE_STATE_DEPTH_READ,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
};*/

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
//         [ ] single large heap
//         [ ] how to deal with allocation "scheduling"?
//           * NOTE: large-ish heap with smaller "overflow" heaps?
//           * NOTE: mostly persistent, allow fragmentation?
//           * NOTE: manual flag on adding pass?
//         [ ] get physical size of resources
//           * NOTE: alignment?
//         [ ] fix proper creation descriptions 
//          

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
struct FrameGraphResource(&'static str, u32);

trait ResourceBinding {
    type PhysicalResource;

    fn get_virtual_resource(&self) -> FrameGraphResource;
}


macro_rules! physical_resource_bind {
    ($name:ident => $physical:ty) => {
        struct $name(FrameGraphResource);

        impl ResourceBinding for $name {
            type PhysicalResource = $physical;

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

trait IntoTypedResource<T> {
    fn get_virtual_resource(&self) -> FrameGraphResource;
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

struct FrameGraph {
    renderpasses: Vec<RenderPass>,
    renderpass_transitions: Vec<Vec<ResourceTransition>>,

    resources: Vec<(u32, TransitionFlags, Option<usize>, Option<usize>)>,

    virtual_offset: u32
}

impl FrameGraph {
    pub fn new() -> Self {
        FrameGraph {
            renderpasses: Vec::new(),
            renderpass_transitions: Vec::new(),
            resources: Vec::new(),
            virtual_offset: 0,
        }
    }

    pub fn add_pass<T, Init, Exec>(&mut self, name: &'static str, init: Init, exec: Exec) -> T
        where T: Sized /*+ Copy + Clone */,
              Init: FnOnce(&mut FrameGraphBuilder) -> T,
              Exec: FnMut(T)
    {
        let mut builder = FrameGraphBuilder::new(self.virtual_offset);

        let output = init(&mut builder);

        self.virtual_offset = builder.counter;

        self.resources.extend(builder.created.into_iter().map(|(a,b)|(a, b, None, None)));

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
                    self.resources[resource.0 as usize].0 += 1;
                } else if resource.1.has_write() {
                    pass.refcount += 1;
                }
            }
        }

        use ::std::collections::vec_deque::VecDeque;

        // push all resources that are never read to stack
        let mut unused = VecDeque::new();
        for (idx, resource) in self.resources.iter().enumerate() {
            if resource.0 == 0 {
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
                    prune_list.push(self.resources[resource].2.unwrap());

                    for resource in &producer.resources {
                        if resource.1.has_read() {
                            self.resources[resource.0 as usize].0 -= 1;
                            if self.resources[resource.0 as usize].0 == 0 {
                                unused.push_back(resource.0 as usize);
                            }
                        }
                    }
                } 
            } else {
                break;
            }
        }

        // remove our culled passes
        for i in prune_list {
            self.renderpasses.remove(i);
        }
    }

    fn find_lifetimes(&mut self) {
        // find first and last usage of a resource
        for (idx, mut resource) in self.resources.iter_mut().enumerate() {
            let first_use = self.renderpasses.iter().position(|pass| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());
            let last_use = self.renderpasses.iter().rposition(|pass| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());

            resource.2 = first_use;
            resource.3 = last_use;
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

        //println!("Resources: {:#?}", self.resources);
        //println!("Renderpasses: {:#?}", self.renderpasses);
        println!("Transitions: {:#?}", self.renderpass_transitions);
    }

    pub fn dump(&mut self) {
    }
}

// TODO: each placed resource needs to be created with all usages known
//       ahead of time (RT, DT) and also be able to transition into proper
//       states (SRV, RTV, DSV)
//
// TODO: heap only requires `HEAP_ALLOW_RT_DS`?
//
// TODO: strongly typed graph resources? add another i32 for tracking state?
//       what about views? bake into typed resources at setup-phase?
#[derive(Debug)]
struct FrameGraphBuilder {
    created: Vec<(u32, TransitionFlags)>,
    resources: Vec<(u32, TransitionFlags)>,
    counter: u32,
}

impl FrameGraphBuilder {
    fn new(offset: u32) -> Self {
        FrameGraphBuilder {
            created: Vec::new(),
            resources: Vec::new(),
            counter: offset
        }
    }

    fn create_render_target(&mut self, name: &'static str, desc: RenderTargetDesc) -> RenderTargetResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);
        self.created.push((virtual_id, TransitionFlags::RENDER_TARGET));
        self.write(res, TransitionFlags::RENDER_TARGET);

        RenderTargetResource(res)
    }

    fn create_depth(&mut self, name: &'static str, desc: DepthDesc) -> DepthWriteResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);
        self.created.push((virtual_id, TransitionFlags::DEPTH_WRITE));
        self.write(res, TransitionFlags::DEPTH_WRITE);

        DepthWriteResource(res)
    }

    fn read_srv<T: IntoTypedResource<ShaderResource>>(&mut self, resource: &T) -> ShaderResource {
        ShaderResource(self.read(resource.get_virtual_resource(), TransitionFlags::SHADER_RESOURCE))
    }

    fn read_depth<T: IntoTypedResource<DepthReadResource>>(&mut self, resource: &T) -> DepthReadResource {
        DepthReadResource(self.read(resource.get_virtual_resource(), TransitionFlags::DEPTH_READ))
    }

    fn write_depth<T: IntoTypedResource<DepthWriteResource>>(&mut self, resource: T) -> DepthWriteResource {
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




#[derive(Debug)]
enum TextureSize {
    Full,
    Half,
    Explicit(u32, u32)
}

#[derive(Debug)]
enum InitialResourceState {
    Clear,
    DontCare
}

#[derive(Debug)]
enum ResourceState {
    Clear,
    DontCare
}

#[derive(Debug)]
enum DepthFormat {
    D32,
    D24
}

#[derive(Debug)]
struct DepthDesc {
    format: DepthFormat,
    size: TextureSize,
    state: InitialResourceState,
}

#[derive(Debug)]
enum TextureFormat {
    RGBA8,
    R8,
}

#[derive(Debug)]
struct RenderTargetDesc {
    format: TextureFormat,
    size: TextureSize,
    mip_levels: u32,
    state: InitialResourceState,
}


fn main() {
    let mut fg = FrameGraph::new();

    // early depth
    let depth = fg.add_pass(
        "EarlyDepth",
        |builder| {
            let desc = DepthDesc {
                format: DepthFormat::D32,
                size: TextureSize::Full,
                state: InitialResourceState::Clear
            };

            builder.create_depth("Depth", desc)
        },
        |depth| {
            
        }
    );

    // ambient occlusion
    let ao = fg.add_pass(
        "SSAO",
        |builder| {
            builder.read_srv(&depth);

            let desc = RenderTargetDesc {
                format: TextureFormat::R8,
                size: TextureSize::Full,
                mip_levels: 1,
                state: InitialResourceState::Clear
            };
            builder.create_render_target("Raw Occlusion", desc)
        },
        |_| {

        }
    );

    let (color, depth, ao) = fg.add_pass(
        "Forward",
        |builder| {
            let depth = builder.read_depth(&depth);
            let ao = builder.read_srv(&ao);

            let desc = RenderTargetDesc {
                format: TextureFormat::RGBA8,
                size: TextureSize::Full,
                mip_levels: 1,
                state: InitialResourceState::Clear
            };

            (builder.create_render_target("Color", desc), depth, ao)
        },
        |_| {

        }
    );

    let _ = fg.add_pass(
        "Wat",
        move |builder| {
            let c = builder.read_srv(&color);
            builder.write_depth(depth);

            c
        },
        |_| {

        }
    );

    fg.compile();
    fg.dump();
}



fn dump_file(path: &str, text: String)  {
    use ::std::fs::File;
    use ::std::io::Write;

    let mut file = File::create(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}
