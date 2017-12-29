use framegraph::{
    ResourceView,
    ResourceViewDesc,
    TransientResource,
    TransientResourceLifetime
};

use winapi::um::d3d12::*;
use winapi::Interface;

use std::ptr;

#[derive(Debug, Copy, Clone)]
pub struct MemoryRegion {
    offset: u64,
    size: u64,
    start: u32,
    end: u32
}

impl MemoryRegion {
    pub fn new(offset: u64, size: u64, start: u32, end: u32) -> MemoryRegion {
        MemoryRegion {
            offset,
            size,
            start,
            end
        }
    }

    pub fn intersects(&self, other: MemoryRegion) -> bool {
        self.start < other.end &&
        self.end > other.start &&
        self.offset < other.offset + other.size &&
        self.offset + self.size > other.offset
    }
}

#[derive(Debug)]
pub struct HeapBin {
    size: u64,
    scanlines: Vec<u64>,
    elements: Vec<MemoryRegion>
}

impl HeapBin {
    pub fn new(size: u64) -> HeapBin {
        HeapBin {
            size,
            scanlines: vec![0u64],
            elements: Vec::new()
        }
    }

    fn clear(&mut self) {
        self.scanlines.clear();
        self.elements.clear();
    }

    fn occupied(&self, newregion: MemoryRegion) -> bool {
        for &region in &self.elements {
            if newregion.intersects(region) {
                return true;
            }
        }

        false
    }

    fn insert(&mut self, lifetime: TransientResourceLifetime, size: u64) -> Option<u64> {
        if size > self.size {
            return None;
        }

        let mut line = None;

        for &offset in &self.scanlines {
            if offset + size > self.size {
                continue;
            }

            let region = MemoryRegion::new(offset, size, lifetime.start, lifetime.end);

            if !self.occupied(region) {
                line = Some((offset, offset + size));
                self.elements.push(region);

                break;
            }
        }

        if let Some((offset, newline)) = line {
            self.scanlines.push(newline);

            Some(offset)
        } else {
            None
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct HeapMemoryCacheEntry {
    hash: u64,
    #[derivative(Debug="ignore")]
    resources: Vec<(u64, TransientResourceLifetime, D3D12_RESOURCE_DESC)>,
    #[derivative(Debug="ignore")]
    views: Vec<ResourceView>,

    placed_resources: Vec<*mut ID3D12Resource>,
    #[derivative(Debug="ignore")]
    gpu_handles: Vec<D3D12_GPU_DESCRIPTOR_HANDLE>,
    #[derivative(Debug="ignore")]
    cpu_handles: Vec<D3D12_CPU_DESCRIPTOR_HANDLE>,
    indices: Vec<(usize, u64)>
}

impl HeapMemoryCacheEntry {
    pub fn new() -> Self {
        HeapMemoryCacheEntry {
            hash: 0u64,
            resources: Vec::new(),
            views: Vec::new(),
            placed_resources: Vec::new(),
            gpu_handles: Vec::new(),
            cpu_handles: Vec::new(),
            indices: Vec::new()
        }
    }

    pub fn get_cpu_handle(&self, id: usize) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        self.cpu_handles[id]
    }

    pub fn get_gpu_handle(&self, id: usize) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        self.gpu_handles[id]
    }
/*
    pub fn find_resource(&self, resource: usize) -> &HeapBin {
        &self.bins[self.indices[resource]]
    }*/
}

#[derive(Debug, Copy, Clone)]
pub struct Heap {
    heap: *mut ID3D12Heap,
    size: u64
}

pub struct HeapLayout {
    heaps: Vec<Heap>
}

impl HeapLayout {
    fn new() -> Self {
        HeapLayout {
            heaps: Vec::new()
        }
    }
}

// TODO: linear array of 0..virtual_id for both created & views?

#[derive(Debug)]
pub struct HeapMemoryAllocator {
    device: *mut ID3D12Device,

    cbv_srv_uav_heap: *mut ID3D12DescriptorHeap,
    srv_stride: u32,

    rtv_dsv_heap: *mut ID3D12DescriptorHeap,
    rtv_stride: u32,

    current_layout: Vec<Heap>,
    cache: [HeapMemoryCacheEntry; 8],
}

impl HeapMemoryAllocator {
    pub fn new(device: *mut ID3D12Device) -> Self {
        let (gpu_heap, gpu_stride, cpu_heap, cpu_stride) = unsafe {
            let mut gpu_heap: *mut ID3D12DescriptorHeap = ptr::null_mut();
            let mut cpu_heap: *mut ID3D12DescriptorHeap = ptr::null_mut();

            let desc = D3D12_DESCRIPTOR_HEAP_DESC {
                NumDescriptors: 3000,
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0
            };

            (*device).CreateDescriptorHeap(&desc, &ID3D12DescriptorHeap::uuidof(), &mut gpu_heap as *mut *mut _ as *mut *mut _);

            let desc = D3D12_DESCRIPTOR_HEAP_DESC {
                NumDescriptors: 3000,
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0
            };

            (*device).CreateDescriptorHeap(&desc, &ID3D12DescriptorHeap::uuidof(), &mut cpu_heap as *mut *mut _ as *mut *mut _);

            let gpu_stride = (*device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
            let cpu_stride = (*device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);

            (gpu_heap, gpu_stride, cpu_heap, cpu_stride)
        };

        HeapMemoryAllocator {
            device: device,

            cbv_srv_uav_heap: gpu_heap,
            srv_stride: gpu_stride,

            rtv_dsv_heap: cpu_heap,
            rtv_stride: cpu_stride,

            current_layout: Vec::new(),
            cache: [HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new(),HeapMemoryCacheEntry::new()]
        }
    }

    pub fn get_cpu_handle(&self, id: usize) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        let mut handle = unsafe { (*self.rtv_dsv_heap).GetCPUDescriptorHandleForHeapStart() };
        handle.ptr += id * self.rtv_stride as usize;

        handle
    }

    pub fn get_gpu_handle(&self, id: u64) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        let mut handle = unsafe { (*self.cbv_srv_uav_heap).GetGPUDescriptorHandleForHeapStart() };
        handle.ptr += id * self.srv_stride as u64;

        handle
    }

    pub fn current(&self) -> &HeapMemoryCacheEntry {
        &self.cache[0]
    }

    fn resize(&mut self, resources: &Vec<(u64, TransientResourceLifetime, D3D12_RESOURCE_DESC)>) {
        // TODO: do we even need to sort *all* resources?
        let mut cached_resources: Vec<(u64, TransientResourceLifetime, D3D12_RESOURCE_DESC)> = Vec::new();
        cached_resources.extend(resources);
        for entry in self.cache.iter() {
            if entry.hash != 0 {
                cached_resources.extend(&entry.resources);
            }
        }

        cached_resources.sort_by(|a, b| (b.0).cmp(&a.0));

        let mut layout: Vec<HeapBin> = Vec::new();

        for entry in self.cache.iter_mut() {
            if entry.hash != 0 {

                entry.indices.resize(entry.resources.len(), (0, 0));

                for bin in &mut layout {
                    bin.clear();
                }

                'r: for (idx, resource) in entry.resources.iter().enumerate() {
                    let mut i = 0;
                    loop {
                        if i >= layout.len() {
                            // TODO: rethink how heap sizes are fetched
                            layout.push(HeapBin::new(entry.resources[i].0));
                        }

                        if let Some(offset) = layout[i].insert(resource.1, resource.0) {
                            entry.indices[idx] = (i, offset);
                            continue 'r;
                        }

                        i += 1;
                    }
                }
            }
        }

        layout.sort_by(|a, b| (b.size).cmp(&a.size));

        let mut existing = vec![None; self.current_layout.len()];
        for (heap_idx, heap) in self.current_layout.iter().enumerate() {
            for (idx, h) in layout.iter().enumerate() {
                if h.size == heap.size {
                    if let Some(i) = existing[heap_idx] {
                        if idx == i {
                            continue;
                        }
                    } else {
                        existing[heap_idx] = Some(idx);
                    }            
                }
            }
        }

        let mut new_heaps = Vec::with_capacity(layout.len());
        for (idx, heap) in self.current_layout.iter().enumerate() {
            if existing[idx].is_some() {
                println!("Carrying Over Heap #{}: {} B", idx, heap.size);
                new_heaps.push(*heap)
            } else {
                println!("Deleting Heap #{}: {} B", idx, heap.size);
                unsafe { (*heap.heap).Release(); }
            }
        }

        // TODO: implement resource aliasing, needs to be done after all resources have
        //       been packed
        for (idx, heap) in layout.iter().enumerate() {
            if existing.iter().find(|&a| if let &Some(i) = a { i == idx } else { false }).is_some() {
                continue;
            }

            let h = unsafe {
                let mut heap_ptr: *mut ID3D12Heap = ::std::mem::zeroed();

                let desc = D3D12_HEAP_DESC {
                    SizeInBytes: heap.size,
                    Properties: D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                        CreationNodeMask: 0,
                        VisibleNodeMask: 0,
                    },
                    Alignment: 0,
                    Flags: D3D12_HEAP_FLAG_ALLOW_ONLY_RT_DS_TEXTURES
                };

                println!("Creating Heap #{}: {} B", idx, heap.size);
                (*self.device).CreateHeap(&desc, &ID3D12Heap::uuidof(), &mut heap_ptr as *mut *mut _ as *mut *mut _);

                heap_ptr
            };

            new_heaps.push(Heap {
                heap: h,
                size: heap.size
            })
        }

        for entry in self.cache.iter_mut() {
            if entry.hash != 0 {
                for (idx, &(heap, offset)) in entry.indices.iter().enumerate() {
                    let mut resource: *mut ID3D12Resource = ptr::null_mut();
                    unsafe {
                        (*self.device).CreatePlacedResource(new_heaps[heap].heap, offset, &entry.resources[idx].2, D3D12_RESOURCE_STATE_RENDER_TARGET, ptr::null_mut(), &ID3D12Resource::uuidof(), &mut resource as *mut *mut _ as *mut *mut _);
                    }

                    // TODO: probably should be in some order?
                    entry.placed_resources.push(resource);
                }
            }
        }

        self.current_layout = new_heaps;
    }

    fn find_entry(&self, hash: u64) -> Option<usize> {
        self.cache.iter().position(|entry| entry.hash == hash)
    }

    fn push_entry(&mut self, hash: u64, resources: Vec<(u64, TransientResourceLifetime, D3D12_RESOURCE_DESC)>, views: &Vec<ResourceView>) -> &HeapMemoryCacheEntry {
        for i in 0..7 {
            self.cache.swap(7 - i, 7 - i - 1);
        }

        {
            self.cache[0].hash = hash;
            self.cache[0].resources.clear();
            self.cache[0].resources.extend(&resources);

            self.resize(&resources);

            for view in views {
                let resource = self.cache[0].placed_resources[view.resource_id as usize];

                match view.desc {
                    ResourceViewDesc::RenderTarget(desc) => {
                        let handle = self.get_cpu_handle(view.view_id as usize);

                        unsafe {
                            (*self.device).CreateRenderTargetView(resource, &desc, handle);
                        }
                    },
                    ResourceViewDesc::ShaderResource(desc) => {

                    }
                }
            }
        }

        &self.cache[0]
    }

    pub fn pack_heap(&mut self, resources: &Vec<TransientResource>, views: &Vec<ResourceView>) -> &HeapMemoryCacheEntry {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        resources.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(entry) = self.find_entry(hash) {
            &self.cache[entry]
        } else {
            let mut resources = resources.iter().map(|r| (r.size, r.lifetime, r.desc)).collect::<Vec<_>>();
            resources.sort_by(|a, b| (b.0).cmp(&a.0));

            self.push_entry(hash, resources, views)
        }
    }

    fn dump(bins: &Vec<HeapBin>) {
        use svg;

        let node = svg::node::element::Rectangle::new()
            .set("x", 20)
            .set("y", 20)
            .set("width", 40)
            .set("height", 40);

        let heap_width = 500f64;
        let heap_height = 80f64;
        let padding = 10f64;

        let mut doc = svg::Document::new();
        let max_height = bins.iter().map(|b|b.size).max().unwrap() as f64;
        let max_width = bins.iter().map(|b|b.elements.iter().map(|e|e.end).max().unwrap_or(0)).max().unwrap() as f64;

        doc = doc.set("stroke", "black").set("stroke-width", 1);

        let mut y = padding;
        for bin in bins {
            let height = bin.size as f64 / max_height as f64 * heap_height;

            let node = svg::node::element::Rectangle::new()
                    .set("x", 0)
                    .set("y", y)
                    .set("width", heap_width)
                    .set("height", height)
                    .set("fill", "transparent")
                    .set("stroke", "black")
                    .set("stroke-width", 1);
            doc = doc.add(node);

            for region in &bin.elements {
                let size = region.size;

                let xoff = region.start as f64 / max_width as f64 * heap_width;
                let yoff = region.offset as f64 / bin.size as f64 * height;
                let w = (region.end - region.start) as f64 / max_width * heap_width;
                let h = region.size as f64 / bin.size as f64 * height;

                let mut node = svg::node::element::Rectangle::new()
                    .set("x", xoff)
                    .set("y", y + yoff)
                    .set("width", w)
                    .set("height", h)
                    .set("fill", "green")
                    .set("stroke", "black")
                    .set("stroke-width", 1);

                let text = svg::node::Text::new("Test");
                let mut title = svg::node::element::Text::new()
                    .set("x", 0)
                    .set("y", 0)
                    .set("font-family", "monospace");
                title = title.add(text);
                node = node.add(title);

                doc = doc.add(node);
            }

            y += height + padding;
        }

        svg::save("memory.svg", &doc).unwrap();
    }

    pub fn alloc(size: usize, begin: u32, end: u32) {

    }
}
