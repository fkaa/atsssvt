#![allow(dead_code)]
#![allow(unused_variables)]

extern crate dot;

type Node = i32;
type Edge = (i32, i32);

struct Graph {
    nodes: Vec<FrameGraphNode>,
    edges: Vec<Edge>
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new()
        }
    }

    pub fn add_node(&mut self, node: FrameGraphNode) -> Node {
        let pos = self.nodes.iter().position(|&n| node == n);

        if let Some(idx) = pos {
            idx as i32
        } else {
            let idx = self.nodes.len() as Node;
            self.nodes.push(node);
            idx
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }
}

impl<'a> dot::Labeller<'a, Node, Edge> for Graph {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("test").unwrap()
    }

    fn node_id(&'a self, node: &Node) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", node)).unwrap()
    }

    fn node_label<'b>(&'b self, node: &Node) -> dot::LabelText<'b> {
        match self.nodes[*node as usize] {
            FrameGraphNode::Pass(name) => {
                dot::LabelText::LabelStr(format!("{}", name).into())
            },
            FrameGraphNode::Resource(id) => {
                dot::LabelText::LabelStr(format!("{:?}", id).into())
            }
        }
    }

    fn edge_label<'b>(&'b self, edge: &Edge) -> dot::LabelText<'b> {
        if let FrameGraphNode::Pass(_) = self.nodes[edge.0 as usize] {
            dot::LabelText::LabelStr("Write".into())
        } else {
            dot::LabelText::LabelStr("Read".into())
        }
    }

    fn node_shape(&'a self, node: &Node) -> Option<dot::LabelText<'a>> {
        if let FrameGraphNode::Pass(_) = self.nodes[*node as usize] {
            Some(dot::LabelText::LabelStr("box".into()))
        } else {
            Some(dot::LabelText::LabelStr("box".into()))
        }
    }

    fn node_style(&'a self, node: &Node) -> dot::Style {
        if let FrameGraphNode::Pass(_) = self.nodes[*node as usize] {
            dot::Style::Rounded
        } else {
            dot::Style::None
        }

    }
}

impl<'a> dot::GraphWalk<'a, Node, Edge> for Graph {
    fn nodes(&self) -> dot::Nodes<'a, Node> { ::std::borrow::Cow::Owned((0..(self.nodes.len() as i32)).collect()) }
    fn edges(&'a self) -> dot::Edges<'a, Edge> { ::std::borrow::Cow::Owned(self.edges.iter().map(|e|*e).collect()) }
    fn source(&self, e: &Edge) -> Node { let &(s,_) = e; s }
    fn target(&self, e: &Edge) -> Node { let &(_,t) = e; t }
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
struct FrameGraphResource(i32);

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum FrameGraphNode {
    Pass(&'static str),
    Resource(FrameGraphResource),
}

struct FrameGraph {
    graph: Graph
    //current_pass: FrameGraphPass
}

impl FrameGraph {
    pub fn new() -> Self {
        FrameGraph {
            graph: Graph::new(),
            //current_pass: 
        }
    }

    pub fn add_pass<T, Init, Exec>(&mut self, name: &'static str, mut init: Init, exec: Exec) -> T
        where T: Sized + Copy + Clone,
              Init: FnMut(&mut FrameGraphBuilder) -> T,
              Exec: FnMut(T)
    {
        let mut builder = FrameGraphBuilder::new();

        let output = init(&mut builder);

        let pass = self.graph.add_node(FrameGraphNode::Pass(name));

        for &resource in &builder.input {
            let resource = self.graph.add_node(FrameGraphNode::Resource(resource));
            self.graph.add_edge((resource, pass));
        }

        for &resource in &builder.output {
            let resource = self.graph.add_node(FrameGraphNode::Resource(resource));
            self.graph.add_edge((pass, resource));
        }

        println!("{:?}: {:#?}", name, builder);

        output
    }

    pub fn cull(&mut self) {

    }

    pub fn dump(&self) {
        let mut file = ::std::fs::File::create("graph.dot").unwrap();
        dot::render(&self.graph, &mut file).unwrap();
        //dump_file("graph.dot", format!("{:?}", petgraph::dot::Dot::new(&self.graph)));
    }
}

enum FrameGraphDepthFormat {
    D32,
    D24
}

enum FrameGraphInitialState {
    Clear,
    DontCare
}

struct FrameGraphDepthDesc {
    format: FrameGraphDepthFormat,
    width: u32,
    height: u32,
    state: FrameGraphInitialState,
}

// we know every pass read/write for resources, thus giving us high-level view
// of all transitions from read <=> write of all resources
//
// TODO: strongly typed graph resources? add another i32 for tracking state?
//       what about views? bake into typed resources at setup-phase?
#[derive(Debug)]
struct FrameGraphBuilder {
    input: Vec<FrameGraphResource>,
    output: Vec<FrameGraphResource>
}

impl FrameGraphBuilder {
    fn new() -> Self {
        FrameGraphBuilder {
            input: Vec::new(),
            output: Vec::new()
        }
    }

    fn create_depth(&mut self, desc: FrameGraphDepthDesc) -> FrameGraphResource {
        FrameGraphResource(1)
    }

    fn read(&mut self, resource: FrameGraphResource) -> FrameGraphResource {
        self.input.push(resource);
        resource
    }

    fn write(&mut self, resource: FrameGraphResource) -> FrameGraphResource {
        self.output.push(resource);
        resource
    }
}

fn main() {
    let mut fg = FrameGraph::new();

    let depth = FrameGraphResource(0);

    // early depth
    let depth = fg.add_pass(
        "EarlyDepthPass",
        |builder| {
            /*let desc = FrameGraphDepthDesc {

            };

            builder.create_depth(desc)*/
            builder.write(depth)
        },
        |depth| {
            
        }
    );

    let occlusion = FrameGraphResource(1);
    // ambient occlusion
    let ao = fg.add_pass(
        "AmbientOcclusionPass",
        |builder| {
            builder.read(depth);
            builder.write(occlusion);
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
