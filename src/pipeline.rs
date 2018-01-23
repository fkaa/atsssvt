use winapi::um::d3d12::*;
use winapi::um::d3d12sdklayers::*;
use winapi::um::d3dcommon::*;

use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::winerror::*;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgi1_3::*;
use winapi::shared::dxgi1_4::*;

use winapi::Interface;

use std::path::Path;
use std::ffi::OsString;
use std::ffi::CString;
use std::os::windows::ffi::OsStringExt;

pub struct ShaderBlob {
    bytecode: Box<[u8]>
}

impl ShaderBlob {
    pub fn from_file<T: AsRef<Path>>(path: T) -> ShaderBlob {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path.as_ref()).unwrap();

        let mut contents: Vec<u8> = Vec::new();
        let result = file.read_to_end(&mut contents).unwrap();

        ShaderBlob {
            bytecode: contents.into_boxed_slice()
        }
    }
}

impl ShaderBlob {
    fn as_d3d12(&self) -> D3D12_SHADER_BYTECODE {
        D3D12_SHADER_BYTECODE {
            pShaderBytecode: self.bytecode.as_ptr() as _,
            BytecodeLength: self.bytecode.len()
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum Blend {
    Zero = 1,
    One = 2,
    SrcColor = 3,
    InvSrcColor = 4,
    SrcAlpha = 5,
    InvSrcAlpha = 6,
    DestAlpha = 7,
    InvDestAlpha = 8,
    DestColor = 9,
    InvDestColor = 10,
    SrcAlphaSat = 11,
    BlendFactor = 14,
    InvBlendFactor = 15,
    Src1Color = 16,
    InvSrc1Color = 17,
    Src1Alpha = 18,
    InvSrc1Alpha = 19
}

impl Into<D3D12_BLEND> for Blend {
    fn into(self) -> D3D12_BLEND {
        match self {
            Blend::Zero => D3D12_BLEND_ZERO,
            Blend::One => D3D12_BLEND_ZERO,
            Blend::SrcColor => D3D12_BLEND_SRC_COLOR,
            Blend::InvSrcColor => D3D12_BLEND_INV_SRC_COLOR,
            Blend::SrcAlpha => D3D12_BLEND_SRC_ALPHA,
            Blend::InvSrcAlpha => D3D12_BLEND_INV_SRC_ALPHA,
            Blend::DestAlpha => D3D12_BLEND_DEST_ALPHA,
            Blend::InvDestAlpha => D3D12_BLEND_INV_DEST_ALPHA,
            Blend::DestColor => D3D12_BLEND_DEST_COLOR,
            Blend::InvDestColor => D3D12_BLEND_INV_DEST_COLOR,
            Blend::SrcAlphaSat => D3D12_BLEND_SRC_ALPHA_SAT,
            Blend::BlendFactor => D3D12_BLEND_BLEND_FACTOR,
            Blend::InvBlendFactor => D3D12_BLEND_INV_BLEND_FACTOR,
            Blend::Src1Color => D3D12_BLEND_SRC1_COLOR,
            Blend::InvSrc1Color => D3D12_BLEND_INV_SRC1_COLOR,
            Blend::Src1Alpha => D3D12_BLEND_SRC1_ALPHA,
            Blend::InvSrc1Alpha => D3D12_BLEND_INV_SRC1_ALPHA,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum BlendOp {
    Add = 1,
    Subtract = 2,
    RevSubtract = 3,
    Min = 4,
    Max = 5,
}

impl Into<D3D12_BLEND_OP> for BlendOp {
    fn into(self) -> D3D12_BLEND_OP {
        match self {
            BlendOp::Add => D3D12_BLEND_OP_ADD,
            BlendOp::Subtract => D3D12_BLEND_OP_SUBTRACT,
            BlendOp::RevSubtract => D3D12_BLEND_OP_REV_SUBTRACT,
            BlendOp::Min => D3D12_BLEND_OP_MIN,
            BlendOp::Max => D3D12_BLEND_OP_MAX,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum LogicOp {
    Clear = 0,
    Set = 1,
    Copy = 2,
    CopyInverted = 3,
    Noop = 4,
    Invert = 5,
    And = 6,
    Nand = 7,
    Or = 8,
    Nor = 9,
    Xor = 11,
    Equiv = 12,
    AndReverse = 13,
    AndInverted = 14,
    OrReverse = 15,
    OrInverted = 16,
}

impl Into<D3D12_LOGIC_OP> for LogicOp {
    fn into(self) -> D3D12_LOGIC_OP {
        match self {
            LogicOp::Clear => D3D12_LOGIC_OP_CLEAR,
            LogicOp::Set => D3D12_LOGIC_OP_SET,
            LogicOp::Copy => D3D12_LOGIC_OP_COPY,
            LogicOp::CopyInverted => D3D12_LOGIC_OP_COPY_INVERTED,
            LogicOp::Noop => D3D12_LOGIC_OP_NOOP,
            LogicOp::Invert => D3D12_LOGIC_OP_INVERT,
            LogicOp::And => D3D12_LOGIC_OP_AND,
            LogicOp::Nand => D3D12_LOGIC_OP_NAND,
            LogicOp::Or => D3D12_LOGIC_OP_OR,
            LogicOp::Nor => D3D12_LOGIC_OP_NOR,
            LogicOp::Xor => D3D12_LOGIC_OP_XOR,
            LogicOp::Equiv => D3D12_LOGIC_OP_EQUIV,
            LogicOp::AndReverse => D3D12_LOGIC_OP_AND_REVERSE,
            LogicOp::AndInverted => D3D12_LOGIC_OP_AND_INVERTED,
            LogicOp::OrReverse => D3D12_LOGIC_OP_OR_REVERSE,
            LogicOp::OrInverted => D3D12_LOGIC_OP_OR_INVERTED,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RenderTargetBlendDesc {
    pub blend_enable: bool,
    pub logic_op_enable: bool,
    pub src_blend: Blend,
    pub dst_blend: Blend,
    pub blend_op: BlendOp,
    pub src_blend_alpha: Blend,
    pub dst_blend_alpha: Blend,
    pub blend_op_alpha: BlendOp,
    pub logic_op: LogicOp,
    pub write_mask: u8
}

impl Into<D3D12_RENDER_TARGET_BLEND_DESC> for RenderTargetBlendDesc {
    fn into(self) -> D3D12_RENDER_TARGET_BLEND_DESC {
        D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: self.blend_enable as _,
            LogicOpEnable: self.logic_op_enable as _,
            SrcBlend: self.src_blend.into(),
            DestBlend: self.dst_blend.into(),
            BlendOp: self.blend_op.into(),
            SrcBlendAlpha: self.src_blend_alpha.into(),
            DestBlendAlpha: self.dst_blend_alpha.into(),
            BlendOpAlpha: self.blend_op_alpha.into(),
            LogicOp: self.logic_op.into(),
            RenderTargetWriteMask: self.write_mask,
        }
    }
}

#[derive(Copy, Clone)]
pub struct BlendDesc {
    pub alpha_to_coverage: bool,
    pub independent_blend: bool,
    pub render_target: [Option<RenderTargetBlendDesc>; 8]
}

impl BlendDesc {
    fn as_d3d12(&self) -> D3D12_BLEND_DESC {
        let null_rtb: D3D12_RENDER_TARGET_BLEND_DESC = unsafe { ::std::mem::zeroed() };

        D3D12_BLEND_DESC {
            AlphaToCoverageEnable: self.alpha_to_coverage as _,
            IndependentBlendEnable: self.independent_blend as _,
            RenderTarget: [
                self.render_target[0].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[1].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[2].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[3].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[4].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[5].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[6].map_or(null_rtb, |rtb| rtb.into()),
                self.render_target[7].map_or(null_rtb, |rtb| rtb.into()),
            ]
            //RenderTarget: self.render_target.map(|blend| blend.into()).collect()
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum FillMode {
    Wireframe = 2,
    Solid = 3
}

impl Into<D3D12_FILL_MODE> for FillMode {
    fn into(self) -> D3D12_FILL_MODE {
        match self {
            FillMode::Wireframe => D3D12_FILL_MODE_WIREFRAME,
            FillMode::Solid => D3D12_FILL_MODE_SOLID,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum CullMode {
    None = 1,
    Front = 2,
    Back = 3
}

impl Into<D3D12_CULL_MODE> for CullMode {
    fn into(self) -> D3D12_CULL_MODE {
        match self {
            CullMode::None => D3D12_CULL_MODE_NONE,
            CullMode::Front => D3D12_CULL_MODE_FRONT,
            CullMode::Back => D3D12_CULL_MODE_BACK,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum PrimitiveTopologyType {
    Undefined,
    Point,
    Line,
    Triangle,
    Patch
}

impl Into<D3D12_PRIMITIVE_TOPOLOGY_TYPE> for PrimitiveTopologyType {
    fn into(self) -> D3D12_PRIMITIVE_TOPOLOGY_TYPE{
        match self {
            PrimitiveTopologyType::Undefined => D3D12_PRIMITIVE_TOPOLOGY_TYPE_UNDEFINED,
            PrimitiveTopologyType::Point => D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT,
            PrimitiveTopologyType::Line => D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE,
            PrimitiveTopologyType::Triangle => D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            PrimitiveTopologyType::Patch => D3D12_PRIMITIVE_TOPOLOGY_TYPE_PATCH,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum ConservativeRasterization {
    Off,
    On
}

impl Into<D3D12_CONSERVATIVE_RASTERIZATION_MODE> for ConservativeRasterization {
    fn into(self) -> D3D12_CONSERVATIVE_RASTERIZATION_MODE {
        match self {
            ConservativeRasterization::Off => D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
            ConservativeRasterization::On => D3D12_CONSERVATIVE_RASTERIZATION_MODE_ON
        }
    }
}

pub struct RasterizerDesc {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub front_counter_clockwise: bool,
    pub depth_bias: i32,
    pub depth_bias_clamp: f32,
    pub slope_scaled_depth_bias: f32,
    pub depth_clip_enable: bool,
    pub multisample_enable: bool,
    pub antialiased_line_enable: bool,
    pub forced_sample_count: u32,
    pub conservative_raster: ConservativeRasterization
}

impl RasterizerDesc {
    fn as_d3d12(&self) -> D3D12_RASTERIZER_DESC {
        D3D12_RASTERIZER_DESC {
            FillMode: self.fill_mode.into(),
            CullMode: self.cull_mode.into(),
            FrontCounterClockwise: self.front_counter_clockwise as _,
            DepthBias: self.depth_bias,
            DepthBiasClamp: self.depth_bias_clamp,
            SlopeScaledDepthBias: self.slope_scaled_depth_bias,
            DepthClipEnable: self.depth_clip_enable as _,
            MultisampleEnable: self.multisample_enable as _,
            AntialiasedLineEnable: self.antialiased_line_enable as _,
            ForcedSampleCount: self.forced_sample_count,
            ConservativeRaster: self.conservative_raster.into(),
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum DepthWriteMask {
    Zero = 0,
    All = 1,
}

impl Into<D3D12_DEPTH_WRITE_MASK> for DepthWriteMask {
    fn into(self) -> D3D12_DEPTH_WRITE_MASK {
        match self {
            DepthWriteMask::Zero => D3D12_DEPTH_WRITE_MASK_ZERO,
            DepthWriteMask::All => D3D12_DEPTH_WRITE_MASK_ALL
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum ComparisonFunc {
    Never = 1,
    Less = 2,
    Equal = 3,
    LessEqual = 4,
    Greater = 5,
    NotEqual = 6,
    GreaterEqual = 7,
    Always = 8
}

impl Into<D3D12_COMPARISON_FUNC> for ComparisonFunc {
    fn into(self) -> D3D12_COMPARISON_FUNC {
        match self {
            ComparisonFunc::Never => D3D12_COMPARISON_FUNC_NEVER,
            ComparisonFunc::Less => D3D12_COMPARISON_FUNC_LESS,
            ComparisonFunc::Equal => D3D12_COMPARISON_FUNC_EQUAL,
            ComparisonFunc::LessEqual => D3D12_COMPARISON_FUNC_LESS_EQUAL,
            ComparisonFunc::Greater => D3D12_COMPARISON_FUNC_GREATER,
            ComparisonFunc::NotEqual => D3D12_COMPARISON_FUNC_NOT_EQUAL,
            ComparisonFunc::GreaterEqual => D3D12_COMPARISON_FUNC_GREATER_EQUAL,
            ComparisonFunc::Always => D3D12_COMPARISON_FUNC_ALWAYS,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum StencilOp {
    Keep = 1,
    Zero = 2,
    Replace = 3,
    IncrSat = 4,
    DecrSat = 5,
    Invert = 6,
    Incr = 7,
    Decr = 8
}

impl Into<D3D12_STENCIL_OP> for StencilOp {
    fn into(self) -> D3D12_STENCIL_OP {
        match self {
            StencilOp::Keep => D3D12_STENCIL_OP_KEEP,
            StencilOp::Zero => D3D12_STENCIL_OP_ZERO,
            StencilOp::Replace => D3D12_STENCIL_OP_REPLACE,
            StencilOp::IncrSat => D3D12_STENCIL_OP_INCR_SAT,
            StencilOp::DecrSat => D3D12_STENCIL_OP_DECR_SAT,
            StencilOp::Invert => D3D12_STENCIL_OP_INVERT,
            StencilOp::Incr => D3D12_STENCIL_OP_INCR,
            StencilOp::Decr => D3D12_STENCIL_OP_DECR,
        }
    }
}

#[derive(Copy, Clone)]
pub struct DepthStencilOpDesc {
    pub stencil_fail_op: StencilOp,
    pub stencil_depth_fail_op: StencilOp,
    pub stencil_pass_op: StencilOp,
    pub stencil_func: ComparisonFunc,
}

impl DepthStencilOpDesc {
    pub fn disabled() -> Self {
        DepthStencilOpDesc {
            stencil_fail_op: StencilOp::Keep,
            stencil_depth_fail_op: StencilOp::Keep,
            stencil_pass_op: StencilOp::Keep,
            stencil_func: ComparisonFunc::Never
        }
    }
}

impl DepthStencilOpDesc {
    fn as_d3d12(&self) -> D3D12_DEPTH_STENCILOP_DESC {
        D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: self.stencil_fail_op.into(),
            StencilDepthFailOp: self.stencil_depth_fail_op.into(),
            StencilPassOp: self.stencil_pass_op.into(),
            StencilFunc: self.stencil_func.into(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct DepthStencilDesc {
    pub depth_enable: bool,
    pub depth_write_mask: DepthWriteMask,
    pub depth_func: ComparisonFunc,
    pub stencil_enable: bool,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front_face: DepthStencilOpDesc,
    pub back_face: DepthStencilOpDesc
}

impl DepthStencilDesc {
    pub fn disabled() -> Self {
        DepthStencilDesc {
            depth_enable: false,
            depth_write_mask: DepthWriteMask::Zero,
            depth_func: ComparisonFunc::Never,
            stencil_enable: false,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
            front_face: DepthStencilOpDesc::disabled(),
            back_face: DepthStencilOpDesc::disabled(),
        }
    }
}

impl DepthStencilDesc {
    fn as_d3d12(&self) -> D3D12_DEPTH_STENCIL_DESC {
        D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: self.depth_enable as _,
            DepthWriteMask: self.depth_write_mask.into(),
            DepthFunc: self.depth_func.into(),
            StencilEnable: self.stencil_enable as _,
            StencilReadMask: self.stencil_read_mask,
            StencilWriteMask: self.stencil_write_mask,
            FrontFace: self.front_face.as_d3d12(),
            BackFace: self.back_face.as_d3d12()
        }
    }
}

#[derive(Copy, Clone)]
pub enum InputClassification {
    PerVertexData = 0,
    PerInstanceData = 1,
}

impl Into<D3D12_INPUT_CLASSIFICATION> for InputClassification {
    fn into(self) -> D3D12_INPUT_CLASSIFICATION {
        match self {
            InputClassification::PerVertexData => D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
            InputClassification::PerInstanceData => D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
        }
    }
}

pub struct InputElementDesc {
    pub semantic_name: CString,
    pub semantic_index: u32,
    pub format: DXGI_FORMAT,
    pub input_slot: u32,
    pub aligned_byte_offset: u32,
    pub input_slot_class: InputClassification,
    pub instance_data_step_rate: u32

}

impl InputElementDesc {
    pub fn new(name: String, index: u32, format: DXGI_FORMAT, slot: u32, offset: u32, class: InputClassification, step_rate: u32) -> Self {
        InputElementDesc {
            semantic_name: CString::new(name).unwrap(),
            semantic_index: index,
            format: format,
            input_slot: slot,
            aligned_byte_offset: offset,
            input_slot_class: class,
            instance_data_step_rate: step_rate
        }
    }
}

pub struct InputLayoutDesc {
    pub elements: Vec<InputElementDesc>
}

impl InputLayoutDesc {
    pub fn as_d3d12(&self) -> Vec<D3D12_INPUT_ELEMENT_DESC> {
        self.elements.iter().map(|desc| D3D12_INPUT_ELEMENT_DESC {
            SemanticName: desc.semantic_name.as_ptr(),
            SemanticIndex: desc.semantic_index,
            Format: desc.format,
            InputSlot: desc.input_slot,
            AlignedByteOffset: desc.aligned_byte_offset,
            InputSlotClass: desc.input_slot_class.into(),
            InstanceDataStepRate: desc.instance_data_step_rate
        }).collect()
    }
}

pub enum DescriptorRangeType {
    Srv,
    Uav,
    Cbv,
    Sampler
}

pub struct DescriptorRange {
    ty: DescriptorRangeType,
    len: u32,
    base: u32,
    space: u32,
    offset: u32
}

pub enum RootParameter {
    DescriptorTable(Vec<DescriptorRange>),
    Constants(u32, u32, u32),
    Descriptor(u32, u32)
}

pub struct RootSignature {
    parameters: Vec<RootParameter>
}

#[derive(Debug)]
pub struct Factory {
    pub factory: *mut IDXGIFactory4
}

impl Factory {
    pub fn new(debug: bool) -> Result<Self, D3D12Error> {
        let flags = if debug {
            DXGI_CREATE_FACTORY_DEBUG
        } else {
            0
        };

        let mut factory: *mut IDXGIFactory4 = ::std::ptr::null_mut();
        let hr = unsafe { CreateDXGIFactory2(flags, &IDXGIFactory1::uuidof(), &mut factory as *mut *mut _ as *mut *mut _) };

        check_d3d12_hresult(hr)?;

        Ok(Factory {
            factory
        })
    }

    pub fn iter_adapters(&self) -> AdapterIterator {
        AdapterIterator {
            factory: self.factory,
            adapter: ::std::ptr::null_mut(),
            idx: 0,
        }
    }
}

#[derive(Debug)]
pub struct AdapterDescription {
    description: String,
    vendor_id: u32,
    device_id: u32,
    sub_sys_id: u32,
    revision: u32,
    dedicated_video_memory: usize,
    dedicated_system_memory: usize,
    shared_system_memory: usize,
}

#[derive(Debug)]
pub struct Adapter {
    adapter: *mut IDXGIAdapter1,
}

impl Adapter {
    fn from_raw(adapter: *mut IDXGIAdapter1) -> Self {
        Adapter {
            adapter
        }
    }

    pub fn description(&self) -> AdapterDescription {
        unsafe {
            let mut desc: DXGI_ADAPTER_DESC1 = ::std::mem::uninitialized();
            if !SUCCEEDED((*self.adapter).GetDesc1(&mut desc)) {
                unreachable!()
            }

            AdapterDescription {
                description: OsString::from_wide(&desc.Description).into_string().unwrap(),
                vendor_id: desc.VendorId,
                device_id: desc.DeviceId,
                sub_sys_id: desc.SubSysId,
                revision: desc.Revision,
                dedicated_video_memory: desc.DedicatedVideoMemory,
                dedicated_system_memory: desc.DedicatedSystemMemory,
                shared_system_memory: desc.SharedSystemMemory,
            }
        }
    }
}

pub struct AdapterIterator {
    factory: *mut IDXGIFactory4,
    adapter: *mut IDXGIAdapter1,
    idx: u32,
}

impl Iterator for AdapterIterator {
    type Item = Adapter;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let result = (*self.factory).EnumAdapters1(self.idx, &mut self.adapter as _);

            self.idx = self.idx + 1;

            match result {
                S_OK => Some(Adapter::from_raw(self.adapter)),
                DXGI_ERROR_NOT_FOUND => None,
                _ => unreachable!()
            }
        }
    }
}

#[derive(Debug)]
pub struct Device {
    pub device: *mut ID3D12Device,
}

#[derive(Debug, Copy, Clone)]
pub enum D3D12Error {
    Unknown(HRESULT)
}

fn check_d3d12_hresult(hr: HRESULT) -> Result<(), D3D12Error> {
    if !SUCCEEDED(hr) {
        Err(D3D12Error::Unknown(hr))
    } else {
        Ok(())
    }
}

pub unsafe fn enable_debug_layer() {
    let mut debug_controller: *mut ID3D12Debug = ::std::ptr::null_mut();
    if SUCCEEDED(D3D12GetDebugInterface(&ID3D12Debug::uuidof(), ::std::mem::transmute(&mut debug_controller))) {
        (*debug_controller).EnableDebugLayer();
    }
}

impl Device {
    pub fn from_adapter(adapter: Adapter) -> Result<Device, D3D12Error> {
        let mut device: *mut ID3D12Device = ::std::ptr::null_mut();
        let hr = unsafe {
            D3D12CreateDevice(
                ::std::mem::transmute(adapter.adapter.as_mut()),
                D3D_FEATURE_LEVEL_11_0,
                &ID3D12Device::uuidof(),
                &mut device as *mut *mut _ as *mut *mut _
            )
        };

        check_d3d12_hresult(hr)?;

        Ok(Device {
            device
        })
    }

    pub fn create_graphics_pipeline(&self, desc: &GraphicsPipelineDescription) -> Result<GraphicsPipeline, D3D12Error> {
        let null_shader = D3D12_SHADER_BYTECODE {
            pShaderBytecode: ::std::ptr::null_mut(),
            BytecodeLength: 0
        };

        let layout = desc.input_layout.as_d3d12();

        let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            pRootSignature: ::std::ptr::null_mut(),
            VS: desc.vertex_shader.as_d3d12(),
            PS: desc.pixel_shader.as_ref().map_or(null_shader, |s| s.as_d3d12()),
            DS: desc.domain_shader.as_ref().map_or(null_shader, |s| s.as_d3d12()),
            HS: desc.hull_shader.as_ref().map_or(null_shader, |s| s.as_d3d12()),
            GS: desc.geometry_shader.as_ref().map_or(null_shader, |s| s.as_d3d12()),
            StreamOutput: unsafe { ::std::mem::zeroed() },
            BlendState: desc.blend_state.as_d3d12(),
            SampleMask: desc.sample_mask,
            RasterizerState: desc.rasterizer_state.as_d3d12(),
            DepthStencilState: desc.depth_stencil_state.as_d3d12(),
            InputLayout: D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: layout.as_ptr(),
                NumElements: layout.len() as _
            },
            IBStripCutValue: unsafe { ::std::mem::zeroed() },
            PrimitiveTopologyType: desc.primitive_topology_type.into(),
            NumRenderTargets: desc.render_targets.iter().position(|&format| format == DXGI_FORMAT_UNKNOWN).unwrap_or(8) as u32,
            RTVFormats: desc.render_targets,
            DSVFormat: desc.dsv_format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            NodeMask: 0,
            CachedPSO: unsafe { ::std::mem::zeroed() },
            Flags: unsafe { ::std::mem::zeroed() }
        };

        let mut pipeline: *mut ID3D12PipelineState = ::std::ptr::null_mut();
        let hr = unsafe {
            (*self.device).CreateGraphicsPipelineState(
                &desc,
                &ID3D12PipelineState::uuidof(),
                &mut pipeline as *mut *mut _ as *mut *mut _
            )
        };
        check_d3d12_hresult(hr)?;

        Ok(GraphicsPipeline {
            pipeline
        })
    }
}

pub struct GraphicsPipeline {
    pipeline: *mut ID3D12PipelineState
}

pub struct GraphicsPipelineDescription {
    //root_signature: RootSignature,
    pub vertex_shader: ShaderBlob,
    pub pixel_shader: Option<ShaderBlob>,
    pub domain_shader: Option<ShaderBlob>,
    pub hull_shader: Option<ShaderBlob>,
    pub geometry_shader: Option<ShaderBlob>,
    // stream_output
    pub blend_state: BlendDesc,
    pub sample_mask: u32,
    pub rasterizer_state: RasterizerDesc,
    pub depth_stencil_state: DepthStencilDesc,
    pub input_layout: InputLayoutDesc,
    // ib_strip_cut_value,
    pub primitive_topology_type: PrimitiveTopologyType,
    pub render_targets: [DXGI_FORMAT; 8],
    pub dsv_format: DXGI_FORMAT,
    // sample_desc,
    // node_mask,
    // cached_pso,
    // flags,
}

