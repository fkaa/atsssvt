#![allow(dead_code)]
#![allow(unused_variables)]

extern crate winapi;
extern crate term;
#[macro_use]
extern crate derivative;
extern crate svg;

#[macro_use]
extern crate bitflags;

use winapi::um::d3d12::*;
use winapi::um::d3d12sdklayers::*;
use winapi::um::d3dcommon::*;

use winapi::shared::winerror::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgi1_3::*;
use winapi::shared::dxgi1_4::*;

use winapi::shared::minwindef::*;
use winapi::shared::windef::*;

use winapi::um::libloaderapi::*;
use winapi::um::synchapi::*;
use winapi::um::winbase::*;
use winapi::um::winuser::*;
use winapi::um::winnt::*;

use winapi::Interface;

mod alloc;
mod framegraph;

use framegraph::*;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use std::mem;
use std::ptr;

unsafe extern "system" fn callback(window: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }

    DefWindowProcW(window, msg, wparam, lparam)
}

unsafe fn register_window_class() -> Vec<u16> {
    let class_name: Vec<u16> = OsStr::new("Window Class").encode_wide().chain(Some(0).into_iter()).collect::<Vec<u16>>();

    let class = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        lpfnWndProc: Some(callback),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: GetModuleHandleW(ptr::null()),
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: ptr::null_mut(),
    };

    RegisterClassExW(&class);
    class_name
}

unsafe fn create_window(factory: *mut IDXGIFactory4, queue: *mut ID3D12CommandQueue) -> (HWND, *mut IDXGISwapChain1) {
    let class_name = register_window_class();

    let title: Vec<u16> = OsStr::new("D3D12 [FG]").encode_wide().chain(Some(0).into_iter()).collect::<Vec<u16>>();

    let hwnd = CreateWindowExW(WS_EX_APPWINDOW | WS_EX_WINDOWEDGE, class_name.as_ptr(),
                               title.as_ptr() as LPCWSTR,
                               WS_OVERLAPPEDWINDOW | WS_CLIPSIBLINGS |
                               WS_VISIBLE,
                               CW_USEDEFAULT, CW_USEDEFAULT,
                               CW_USEDEFAULT, CW_USEDEFAULT,
                               ptr::null_mut(), ptr::null_mut(),
                               GetModuleHandleW(ptr::null()),
                               ptr::null_mut());

    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: 800,
        Height: 600,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        Stereo: FALSE,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 3,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
        Flags: 0
    };

    let mut swapchain: *mut IDXGISwapChain1 = ptr::null_mut();
    (*factory).CreateSwapChainForHwnd(
        queue as _,
        hwnd,
        &desc,
        ptr::null_mut(),
        ptr::null_mut(),
        &mut swapchain as *mut *mut _ as *mut *mut _
     );

    (hwnd, swapchain)
}

fn main() {
    let (device, queue, hwnd, swapchain) = unsafe {
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

        let mut queue: *mut ID3D12CommandQueue = ptr::null_mut();
        let desc = D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0
        };
        (*device).CreateCommandQueue(&desc, &ID3D12CommandQueue::uuidof(), &mut queue as *mut *mut _ as *mut *mut _);

        let (hwnd, swapchain) = create_window(factory, queue);

        (device, queue, hwnd, swapchain)
    };

    let (allocator, list) = unsafe {
        let mut allocator: *mut ID3D12CommandAllocator = ptr::null_mut();
        (*device).CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, &ID3D12CommandAllocator::uuidof(), &mut allocator as *mut *mut _ as *mut *mut _);

        let mut list: *mut ID3D12GraphicsCommandList = ptr::null_mut();
        (*device).CreateCommandList(
            0,
            D3D12_COMMAND_LIST_TYPE_DIRECT,
            allocator,
            ptr::null_mut(),
            &ID3D12CommandList::uuidof(),
            &mut list as *mut *mut _ as *mut *mut _
        );
        (*list).Close();

        (allocator, list)
    };

    let (fence, fence_event) = unsafe {
        let mut fence: *mut ID3D12Fence = ptr::null_mut();
        (*device).CreateFence(0, D3D12_FENCE_FLAG_NONE, &ID3D12Fence::uuidof(), &mut fence as *mut *mut _ as *mut *mut _);

        let fence_event = CreateEventW(ptr::null_mut(), FALSE, FALSE, ptr::null_mut());

        (fence, fence_event)
    };

    let mut fence_value = 1u64;

    let mut fg = FrameGraph::new(device);

    unsafe {
        let mut msg = mem::zeroed();
        loop {
            if PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if msg.message == WM_QUIT {
                break;
            }
            

            let color = fg.add_pass(
                "Test",
                |builder| {
                    let desc = RenderTargetDesc {
                        format: TextureFormat::RGBA8,
                        size: TextureSize::Full,
                        mip_levels: 1,
                        state: InitialResourceState::Clear
                    };

                    builder.create_render_target("Color", desc)
                },
                Box::new(|_list, _| {

                })
            );

            let _ = fg.add_pass(
                "Dummy",
                |builder| {
                    builder.read_srv(&color);
                    ()
                },
                Box::new(|_list, _| {

                })
            );

            let color = fg.add_pass(
                "TestAlias",
                |builder| {
                    let desc = RenderTargetDesc {
                        format: TextureFormat::RGBA8,
                        size: TextureSize::Full,
                        mip_levels: 1,
                        state: InitialResourceState::Clear
                    };

                    builder.create_render_target("ColorAlias", desc)
                },
                Box::new(|_list, _| {

                })
            );

            let _ = fg.add_pass(
                "DummyAlias",
                |builder| {
                    builder.read_srv(&color);
                    ()
                },
                Box::new(|_list, _| {

                })
            );

            fg.compile();
            
            (*allocator).Reset();
            (*list).Reset(allocator, ptr::null_mut());

            fg.exec(list);

            (*list).Close();
            (*queue).ExecuteCommandLists(1, &list as *const *mut _ as *const *mut _);

            (*swapchain).Present(1, 0);

            fg.finish();

            let old_value = fence_value;
            (*queue).Signal(fence, old_value);
            fence_value += 1;

            if (*fence).GetCompletedValue() < old_value {
                (*fence).SetEventOnCompletion(old_value, fence_event);
                WaitForSingleObject(fence_event, INFINITE);
            }
        }
    }
}



fn dump_file(path: &str, text: String)  {
    use ::std::fs::File;
    use ::std::io::Write;

    let mut file = File::create(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}
