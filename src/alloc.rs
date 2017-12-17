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
    pub fn with_resources(mut resources: Vec<(u64, TransientResourceLifetime)>) -> Vec<HeapBin> {
        resources.sort_by(|a, b| (b.0).cmp(&a.0));
        let mut bins = resources.iter().map(|&(sz, _)| HeapBin::new(sz)).collect::<Vec<HeapBin>>();
        
        'r: for resource in &resources {
            for bin in &mut bins {
                if let Some(_) = bin.insert(resource.1, resource.0) {
                    continue 'r;
                }
            }
        }

        //Self::dump(&bins);

        //println!("{:#?}", bins);
        bins
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
