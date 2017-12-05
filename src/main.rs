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
}

// TODO: restructure according to dice slides:
//         [x] renderpass holds array of used resources (and direction?, R/W)
//         [x] renderpasses is a flat array in framegraph
//         [x] resources also flat array, referenced by renderpasses
//         [/] compile time:
//           [/] iterate over renderpasses:
//             [x] compute resource ref count
//             [x] compute first and last user (renderpass)
//             [/] compute barriers (between renderpasses ?)
//         [ ] culling:
//           [ ] pass.rc += 1 for resource writes
//           [ ] resource.rc += for resource reads
//           [ ] push resources with rc == 0 to stack
//             [ ] while !empty():
//               [ ] pop, resource.producer.rc--
//               [ ] if resource.producer.rc == 0:
//                 [ ] resource.producer.reads.rc--
//                 [ ] if resource.producer.reads.rc == 0
//                   [ ] push to stack

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

struct RenderPass {
    resources: Vec<(u32, TransitionFlags)>,
    refcount: u32
}

struct FrameGraph {
    renderpasses: Vec<(RenderPass, Vec<ResourceTransition>)>,
    resources: Vec<(FrameGraphResource, u32, Option<usize>, Option<usize>)>,
    virtual_offset: u32
}

impl FrameGraph {
    pub fn new() -> Self {
        FrameGraph {
            renderpasses: Vec::new(),
            resources: Vec::new(),
            virtual_offset: 0,
            //current_pass: 
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

        self.renderpasses.push((RenderPass {
            resources: builder.resources,
            refcount: 0
        }, Vec::new()));
        //let pass = self.graph.add_pass(name, builder.transitions);
        
        //for &(resource, transition) in &builder.input {
            //let resource = self.graph.add_resource(resource);
            //self.graph.add_edge((resource, pass, transition));
       // }

        //for &(resource, transition) in &builder.output {
            //let resource = self.graph.add_resource(resource);
            //self.graph.add_edge((pass, resource, transition));
        //}

        output
    }

    pub fn compile(&mut self) {
        for &(ref pass, _) in &self.renderpasses {
            for resource in &pass.resources {
                if resource.1.has_read() {
                    self.resources[resource.0 as usize].1 += 1;
                }
            }
        }

        for (idx, mut resource) in self.resources.iter_mut().enumerate() {
            let first_use = self.renderpasses.iter().position(|&(ref pass, _)| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());
            let last_use = self.renderpasses.iter().rposition(|&(ref pass, _)| pass.resources.iter().find(|res| res.0 == idx as u32).is_some());

            resource.2 = first_use;
            resource.3 = last_use;
        }


        // TODO: do we need multiple vecs to juggle state between passes?
        //       really need prev_state? probably..
        let mut transitions = Vec::with_capacity(self.resources.len());
        for &mut (ref pass, ref mut pass_transitions) in &mut self.renderpasses {
            for resource in &pass.resources {
                let transition = resource.1;

                
                transitions[resource.0 as usize] = resource.1;
            }
        }

/*
        for idx in 0..self.graph.nodes.len() {
            if let &FrameGraphNode::Resource(resource) = &self.graph.nodes[idx] {

                let mut current_state = TransitionFlags::empty();
                let mut prev_state = TransitionFlags::empty();
                let mut cache_node = None;

                for &(from, to, transition) in self.graph.edges.iter() {
                    if idx == to as usize {
                        if let Some(node) = cache_node {
                            if let FrameGraphNode::Pass(name, ref mut transitions) = self.graph.nodes[node as usize] {
                                transitions.push(ResourceTransition {
                                    resource: resource,
                                    from: prev_state,
                                    to: current_state
                                });

                                cache_node = Some(from);
                                prev_state = current_state;
                                current_state = transition;
                            }
                        } else {
                            if prev_state.is_empty() {
                                prev_state = transition;
                            }
                            current_state = transition;
                            cache_node = Some(from);
                        }

                    } else if idx == from as usize {
                        if current_state.intersects(TransitionFlags::RENDER_TARGET | TransitionFlags::DEPTH_WRITE) {
                            cache_node = Some(to);
                            prev_state = current_state;
                            current_state = transition
                        }

                        current_state.insert(transition);
                    }
                }

                if prev_state != current_state {
                    if let Some(node) = cache_node {
                        if let FrameGraphNode::Pass(name, ref mut transitions) = self.graph.nodes[node as usize] {
                            transitions.push(ResourceTransition {
                                resource: resource,
                                from: prev_state,
                                to: current_state
                            });
                        }
                    }
                }
            }
        }



   */
    }

    pub fn dump(&mut self) {
        
        // TODO: states can be one write, multiple read (transition into
        //       required reads in batch and writes in singles)
        //
        /*let mut t = term::stdout().unwrap();

        let names = self.graph.nodes.iter()
            .filter_map(
                |n| {
                    if let &FrameGraphNode::Resource(res) = n {
                        let name = res.0;

                        Some(name.split(' ').map(|s| s.chars().next().unwrap()).collect::<String>())
                    } else {
                        None
                    }
                }
            ).collect::<Vec<String>>();

        let padding = 2;
        let width = names.iter()
            .fold(0,
                |max, n| {
                    max.max(n.len())
                }
            ) + padding;

        t.fg(term::color::BRIGHT_WHITE).unwrap();

        for idx in 0..self.graph.nodes.len() {
            if let &FrameGraphNode::Resource(resource) = &self.graph.nodes[idx] {

                let mut current_state = TransitionFlags::empty();
                let mut prev_state = TransitionFlags::empty();
                let mut cache_node = None;

                for &(from, to, transition) in self.graph.edges.iter() {
                    if idx == to as usize {
                        if let Some(node) = cache_node {
                            if let FrameGraphNode::Pass(name, ref mut transitions) = self.graph.nodes[node as usize] {
                                transitions.push(ResourceTransition {
                                    resource: resource,
                                    from: prev_state,
                                    to: current_state
                                });

                                cache_node = Some(from);
                                prev_state = current_state;
                                current_state = transition;
                            }
                        } else {
                            if prev_state.is_empty() {
                                prev_state = transition;
                            }
                            current_state = transition;
                            cache_node = Some(from);
                        }

                    } else if idx == from as usize {
                        if current_state.intersects(TransitionFlags::RENDER_TARGET | TransitionFlags::DEPTH_WRITE) {
                            cache_node = Some(to);
                            prev_state = current_state;
                            current_state = transition
                        }

                        current_state.insert(transition);
                    }
                }

                if prev_state != current_state {
                    if let Some(node) = cache_node {
                        if let FrameGraphNode::Pass(name, ref mut transitions) = self.graph.nodes[node as usize] {
                            transitions.push(ResourceTransition {
                                resource: resource,
                                from: prev_state,
                                to: current_state
                            });
                        }
                    }
                }
            }
        }


        let mut resources = 0;
        let mut positions = Vec::new();
        let mut created = Vec::new();

        let mut toggle = true;
        for node in &self.graph.nodes {
            if let &FrameGraphNode::Resource(id) = node {
                if toggle {
                    t.fg(term::color::MAGENTA).unwrap();
                } else {
                    t.fg(term::color::BRIGHT_MAGENTA).unwrap();
                }
                write!(t, "{}", names[resources]).unwrap();
                t.reset().unwrap();

                for _ in names[resources].len()..(width + 1) {
                    write!(t, " ").unwrap();
                }

                toggle = !toggle;
                resources += 1;
                positions.push(id);
                created.push(false);
            }
        }
        t.fg(term::color::BRIGHT_WHITE).unwrap();
        write!(t, "   Pass\n").unwrap();

        let nodes = self.graph.nodes.iter();

        for (idx, node) in self.graph.nodes.iter().enumerate() {
            if let &FrameGraphNode::Pass(name, ref transitions) = node {
                for transition in transitions {
                    for i in 0..resources {
                        if created[i] {
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "|").unwrap();
                            t.reset().unwrap();
                        } else {
                            t.fg(term::color::BRIGHT_BLACK).unwrap();
                            write!(t, ":").unwrap();
                        }
                        t.reset().unwrap();

                        for _ in 0..width {
                            write!(t, " ").unwrap();
                        }
                    }
                    write!(t, "\n").unwrap();

                    for &(from, to, _) in &self.graph.edges {
                        if from != idx as i32 && to != idx as i32 {
                            continue;
                        }

                        let res_idx = if from == idx as i32 {
                            to
                        } else if to == idx as i32 {
                            from
                        } else {
                            continue;
                        };

                        let pos = {
                            let r = if let FrameGraphNode::Resource(id) = self.graph.nodes[res_idx as usize] {
                                id 
                            } else {
                                continue;
                            };

                            if r != transition.resource {
                                continue;
                            }

                            positions.iter().position(|&res| res == r).unwrap()
                        };

                        for i in 0..pos {
                            if created[i] {
                                t.bg(term::color::MAGENTA).unwrap();
                                write!(t, "|").unwrap();
                                t.reset().unwrap();
                            } else {
                                write!(t, ":").unwrap();
                            }

                            for _ in 0..width {
                                write!(t, " ").unwrap();
                            }
                        }

                        t.fg(term::color::BLACK).unwrap();
                        t.bg(term::color::BRIGHT_YELLOW).unwrap();
                        write!(t, "#").unwrap();
                        t.bg(term::color::BRIGHT_YELLOW).unwrap();
                        write!(t, "<").unwrap();
                        //t.fg(term::color::BRIGHT_YELLOW).unwrap();
                        for _ in pos..resources {
                            for _ in 0..(width+1) {
                                write!(t, "=").unwrap();
                            }
                        }
                        t.reset().unwrap();
                        
                        t.fg(term::color::BRIGHT_WHITE).unwrap();
                        write!(t, "   Barrier({:?} => {:?})", transition.from, transition.to).unwrap();
                        t.reset().unwrap();
                        write!(t, "\n").unwrap();

                    }
                }

                for &(from, to, _) in &self.graph.edges {
                    if from != idx as i32 && to != idx as i32 {
                        continue;
                    }
                    for i in 0..resources {
                        if created[i] {
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "|").unwrap();
                            t.reset().unwrap();
                        } else {
                            t.fg(term::color::BRIGHT_BLACK).unwrap();
                            write!(t, ":").unwrap();
                        }
                        t.reset().unwrap();

                        for _ in 0..width {
                            write!(t, " ").unwrap();
                        }

                    }
                    write!(t, "\n").unwrap();

                    enum Operation {
                        Read,
                        Write,
                        Create
                    };

                    let (op, res_idx) = if from == idx as i32 {
                        if let Some(n) = self.graph.neighbours_in(to).next() {
                            if n == from {
                                (Operation::Create, to)
                            } else {
                                (Operation::Write, to)
                            }
                        } else {
                            (Operation::Write, to)
                        }
                    } else if to == idx as i32 {
                        (Operation::Read, from)
                    } else {
                        continue;
                    };

                    let pos = {
                        let r = if let FrameGraphNode::Resource(id) = self.graph.nodes[res_idx as usize] {
                            id 
                        } else {
                            continue;
                        };

                        positions.iter().position(|&res| res == r).unwrap()
                    };
                    if let Operation::Create = op {
                        created[pos] = true;
                    }

                    for i in 0..pos {
                        if created[i] {
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "|").unwrap();
                            t.reset().unwrap();
                        } else {
                            write!(t, ":").unwrap();
                        }

                        for _ in 0..width {
                            write!(t, " ").unwrap();
                        }
                    }
                    match op {
                        Operation::Create => {
                            t.fg(term::color::BRIGHT_WHITE).unwrap();
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "@").unwrap();
                            t.bg(term::color::RED).unwrap();
                            write!(t, "<").unwrap();
                            t.fg(term::color::RED).unwrap();
                            for _ in pos..resources {
                                for _ in 0..(width+1) {
                                    write!(t, "-").unwrap();
                                }
                            }
                            t.reset().unwrap();
                        },
                        Operation::Read => {
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "|").unwrap();
                            t.reset().unwrap();

                            t.bg(term::color::GREEN).unwrap();
                            t.fg(term::color::GREEN).unwrap();
                            for _ in pos..resources {
                                for _ in 0..(width+1) {
                                    write!(t, "-").unwrap();
                                }
                            }
                            t.fg(term::color::BRIGHT_WHITE).unwrap();
                            write!(t, ">").unwrap();
                            t.reset().unwrap();
                        },
                        Operation::Write => {
                            t.bg(term::color::MAGENTA).unwrap();
                            write!(t, "|").unwrap();
                            t.reset().unwrap();


                            t.bg(term::color::RED).unwrap();
                            write!(t, "<").unwrap();
                            t.fg(term::color::RED).unwrap();
                            for _ in pos..resources {
                                for _ in 0..(width+1) {
                                    write!(t, "-").unwrap();
                                }
                            }
                            t.reset().unwrap();
                        }
                    };
                    
                    t.fg(term::color::BRIGHT_WHITE).unwrap();
                    write!(t, " {}", name).unwrap();
                    t.reset().unwrap();
                    write!(t, "\n").unwrap();
                }
            }
        }*/
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
    resources: Vec<(u32, TransitionFlags)>,
    counter: u32,
}

impl FrameGraphBuilder {
    fn new(offset: u32) -> Self {
        FrameGraphBuilder {
            resources: Vec::new(),
            counter: offset
        }
    }

    fn create_render_target(&mut self, name: &'static str, desc: RenderTargetDesc) -> RenderTargetResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);
        self.write(res, TransitionFlags::RENDER_TARGET);

        RenderTargetResource(res)
    }

    fn create_depth(&mut self, name: &'static str, desc: DepthDesc) -> DepthWriteResource {
        let virtual_id = self.counter;
        self.counter += 1;
        let res = FrameGraphResource(name, virtual_id);
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

#[derive(Debug, Copy, Clone)]
struct ResourceTransition {
    resource: FrameGraphResource,
    from: TransitionFlags,
    to: TransitionFlags
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

    fg.dump();
}



fn dump_file(path: &str, text: String)  {
    use ::std::fs::File;
    use ::std::io::Write;

    let mut file = File::create(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}
