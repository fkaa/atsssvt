#![allow(dead_code)]
#![allow(unused_variables)]

extern crate winapi;
extern crate term;
extern crate svg;

#[macro_use]
extern crate bitflags;

/*use winapi::um::d3d12::{
    D3D12_RESOURCE_STATES,
    D3D12_RESOURCE_STATE_RENDER_TARGET,
    D3D12_RESOURCE_STATE_DEPTH_WRITE,
    D3D12_RESOURCE_STATE_DEPTH_READ,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
};*/

mod alloc;
mod framegraph;

use framegraph::*;

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
    let (ao_x, ao_y) = fg.add_pass(
        "SSAO",
        |builder| {
            builder.read_srv(&depth);

            let desc = RenderTargetDesc {
                format: TextureFormat::R8,
                size: TextureSize::Full,
                mip_levels: 1,
                state: InitialResourceState::Clear
            };

            (builder.create_render_target("Raw Occlusion #1", desc),
             builder.create_render_target("Raw Occlusion #2", desc))
        },
        |_| {

        }
    );

    let (color, depth, ao_x, ao_y) = fg.add_pass(
        "Forward",
        |builder| {
            let depth = builder.read_depth(&depth);
            let ao_x = builder.read_srv(&ao_x);
            let ao_y = builder.read_srv(&ao_y);

            let desc = RenderTargetDesc {
                format: TextureFormat::RGBA8,
                size: TextureSize::Full,
                mip_levels: 1,
                state: InitialResourceState::Clear
            };

            (builder.create_render_target("Color", desc), depth, ao_x, ao_y)
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
