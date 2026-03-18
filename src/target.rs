pub struct RenderTarget<'a, C, D = ()> {
    pub width: usize,
    pub height: usize,
    pub colors: &'a mut [C],
    pub depths: D,
}

impl<'a, C> RenderTarget<'a, C, ()> {
    pub fn new(width: usize, height: usize, colors: &'a mut [C]) -> Self {
        assert_eq!(
            colors.len(),
            width * height,
            "Color buffer size does not match width * height"
        );
        Self {
            width,
            height,
            colors,
            depths: (),
        }
    }

    pub fn with_depth(self, depths: &'a mut [f32]) -> RenderTarget<'a, C, &'a mut [f32]> {
        assert_eq!(
            depths.len(),
            self.width * self.height,
            "Depth buffer size does not match width * height"
        );
        RenderTarget {
            width: self.width,
            height: self.height,
            colors: self.colors,
            depths,
        }
    }

    pub fn clear_color(&mut self, color: C)
    where
        C: Copy,
    {
        self.colors.fill(color);
    }
}

impl<'a, C> RenderTarget<'a, C, &'a mut [f32]> {
    pub fn clear_color(&mut self, color: C)
    where
        C: Copy,
    {
        self.colors.fill(color);
    }

    pub fn clear_depth(&mut self, depth: f32) {
        self.depths.fill(depth);
    }

    /// Splits the RenderTarget into multiple smaller RenderTargets, each representing a single scanline (row).
    /// This is extremely useful for safely distributing rendering tasks across multiple threads.
    pub fn split_into_scanlines(self) -> Vec<RenderTarget<'a, C, &'a mut [f32]>> {
        self.split_into_horizontal_tiles(1)
    }

    /// Splits the RenderTarget into horizontal bands (tiles) of the specified height.
    pub fn split_into_horizontal_tiles(
        self,
        tile_height: usize,
    ) -> Vec<RenderTarget<'a, C, &'a mut [f32]>> {
        let width = self.width;
        let chunk_size = width * tile_height;

        let color_chunks = self.colors.chunks_mut(chunk_size);
        let depth_chunks = self.depths.chunks_mut(chunk_size);

        color_chunks
            .zip(depth_chunks)
            .map(|(colors, depths)| {
                let actual_height = colors.len() / width;
                RenderTarget {
                    width,
                    height: actual_height,
                    colors,
                    depths,
                }
            })
            .collect()
    }
}

pub struct TileView<'a, C, D = ()> {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub target: RenderTarget<'a, C, D>,
}
