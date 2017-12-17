use framegraph::TransientResourceLifetime;

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

pub struct HeapMemoryAllocator {
    bins: Vec<HeapBin>
}

impl HeapMemoryAllocator {
    pub fn with_resources(mut resources: Vec<(u64, TransientResourceLifetime)>) {
        resources.sort_by(|a, b| (b.0).cmp(&a.0));
        let mut bins = resources.iter().map(|&(sz, _)| HeapBin::new(sz)).collect::<Vec<HeapBin>>();
        
        'r: for resource in &resources {
            for bin in &mut bins {
                if let Some(offset) = bin.insert(resource.1, resource.0) {
                    println!("{:?}", offset);

                    continue 'r;
                }
            }
        }

        println!("{:#?}", bins);
    }

    pub fn alloc(size: usize, begin: u32, end: u32) {

    }
}
