#[repr(Debug, Copy, Clone)]
pub struct MemoryRegion {
    offset: usize,
    size: usize,
    begin: u32,
    end: u32
}

impl MemoryRegion {
    pub fn new(offset: usize, size: usize, begin: u32, end: u32) -> MemoryRegion {
        MemoryRegion {
            offset,
            size,
            begin,
            end
        }
    }
}

#[repr(Debug)]
pub struct HeapBin {
    size: usize,
    elements: Vec<MemoryRegion>
}

impl HeapBin {
    pub fn new(size: usize) -> HeapBin {
        HeapBin {
            size,
            Vec::new()
        }
    }

    fn occupied(&self, lifetime: Range<u32>, size: usize) -> bool {
        for region in self.elements {
            if !(lifetime.end < region.begin || lifetime.start > region.end) {
                return true;
            }


        }

        return false;
    }
}

pub struct HeapMemoryAllocator {
    bins: Vec<HeapBin>
}

// TODO: implement shelf packing, with bins for each possible resource heap
//       size. sort bins from highest to lowest, and insert resources with
//       (x, y, w, h) mapped as (lt.start, offset, lt.end, size). offset is
//       given by waste map. sort resource by size as well?
impl HeapMemoryAllocator {
    pub fn with_resources(resources: Vec<(u32, Range<u32>)) {

    }

    pub fn alloc(size: usize, begin: u32, end: u32) {

    }
}
