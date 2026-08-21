use crate::{
    graphics::{primitive::PrimitiveState, rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexShader},
        varying::Varying,
    },
};

pub fn create_render_pipeline<T, VS, FS>(
    vertex_shader: VS,
    fragment_shader: FS,
    primitive: PrimitiveState<T>,
) -> Pipeline<T, VS, FS>
where
    T: Primitive<VS::Varying>,
    VS: VertexShader,
    FS: FragmentShader<Varying = VS::Varying>,
{
    let rasterizer = T::rasterizer(primitive.front_face, primitive.cull_mode);

    Pipeline::new(rasterizer, vertex_shader, fragment_shader)
}

pub struct Renderer {
    width: usize,
    height: usize,
}

pub struct RenderPass<'pass> {
    render: &'pass Renderer,
}

/// Marker type for no depth buffer
pub struct NoDepth;

/// Marker type for depth buffer enabled
pub struct WithDepth<'a>(&'a mut [f32]);

/// Marker type for no blending
pub struct NoBlend;

/// Marker type for blending enabled
pub struct WithBlend<B>(B);

/// Represents a pipeline bound to a render pass, with compile-time-known depth and blend modes.
///
/// Type parameters:
/// - `T`: Primitive type
/// - `V`: Vertex shader type
/// - `F`: Fragment shader type
/// - `D`: Depth mode (NoDepth or WithDepth)
/// - `B`: Blend mode (NoBlend or WithBlend)
///
/// # Usage
///
/// ```ignore
/// // No depth, no blending
/// pipeline.draw_indexed(vertices, indices, framebuffer, &uniform);
///
/// // With depth
/// pipeline.with_depth(depth_buffer)
///     .draw_indexed(vertices, indices, framebuffer, &uniform);
///
/// // With blending
/// pipeline.with_blend()
///     .draw_indexed(vertices, indices, framebuffer, &uniform);
///
/// // With both
/// pipeline.with_depth(depth_buffer)
///     .with_blend()
///     .draw_indexed(vertices, indices, framebuffer, &uniform);
/// ```
pub struct BoundPipeline<'a, T: Primitive<V::Varying>, V: VertexShader, F, D = NoDepth, B = NoBlend>
{
    renderer: &'a Renderer,
    pipeline: &'a mut Pipeline<T, V, F>,
    depth_mode: D,
    blend_mode: B,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn begin_render_pass(&self) -> RenderPass<'_> {
        RenderPass { render: self }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
    }
}

impl<'pass> RenderPass<'pass> {
    pub fn set_pipeline<'a, T: Primitive<V::Varying>, V: VertexShader, F>(
        &'a self,
        pipeline: &'a mut Pipeline<T, V, F>,
    ) -> BoundPipeline<'a, T, V, F, NoDepth, NoBlend> {
        BoundPipeline {
            renderer: self.render,
            pipeline,
            depth_mode: NoDepth,
            blend_mode: NoBlend,
        }
    }
}

// Methods to transition from NoDepth state
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F, B> BoundPipeline<'a, T, V, F, NoDepth, B> {
    /// Enable depth testing with the provided depth buffer.
    pub fn with_depth(
        self,
        depth_buffer: &'a mut [f32],
    ) -> BoundPipeline<'a, T, V, F, WithDepth<'a>, B> {
        BoundPipeline {
            renderer: self.renderer,
            pipeline: self.pipeline,
            depth_mode: WithDepth(depth_buffer),
            blend_mode: self.blend_mode,
        }
    }
}

// Methods to transition from NoBlend state
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F>
    BoundPipeline<'a, T, V, F, NoDepth, NoBlend>
{
    /// Enable blending (requires bidirectional From/Into conversion).
    #[allow(unused_variables)]
    pub fn with_blend<B>(self, blend: B) -> BoundPipeline<'a, T, V, F, NoDepth, WithBlend<B>> {
        BoundPipeline {
            renderer: self.renderer,
            pipeline: self.pipeline,
            depth_mode: NoDepth,
            blend_mode: WithBlend(blend),
        }
    }
}

impl<'a, T: Primitive<V::Varying>, V: VertexShader, F>
    BoundPipeline<'a, T, V, F, WithDepth<'a>, NoBlend>
{
    /// Enable blending (requires bidirectional From/Into conversion).
    pub fn with_blend<B>(
        self,
        blend: B,
    ) -> BoundPipeline<'a, T, V, F, WithDepth<'a>, WithBlend<B>> {
        BoundPipeline {
            renderer: self.renderer,
            pipeline: self.pipeline,
            depth_mode: self.depth_mode,
            blend_mode: WithBlend(blend),
        }
    }
}

// ============================================================================
// Draw methods for NoDepth + NoBlend
// ============================================================================
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F>
    BoundPipeline<'a, T, V, F, NoDepth, NoBlend>
{
    /// Draw all vertices without depth testing or blending.
    #[inline]
    pub fn draw<Var, U, C>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [C], uniform: &U)
    where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        #[cfg(feature = "rayon")]
        self.pipeline.draw_without_depth(
            vertices,
            framebuffer,
            self.renderer.width,
            self.renderer.height,
            uniform,
        );

        #[cfg(not(feature = "rayon"))]
        self.draw_indexed(vertices, 0..vertices.len(), framebuffer, uniform);
    }

    /// Draw indexed vertices without depth testing or blending.
    #[inline]
    pub fn draw_indexed<Var, U, C>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [C],
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        self.pipeline.draw_indexed_without_depth(
            vertices,
            indexed,
            framebuffer,
            self.renderer.width,
            self.renderer.height,
            uniform,
        );
    }
}

// ============================================================================
// Draw methods for NoDepth + WithBlend
// ============================================================================
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F, B>
    BoundPipeline<'a, T, V, F, NoDepth, WithBlend<B>>
{
    /// Draw all vertices with blending but without depth testing.
    #[inline]
    pub fn draw<Var, U, C, O>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [O], uniform: &U)
    where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
        B: Fn(C, C) -> C + Sync + Copy,
    {
        self.draw_indexed(vertices, 0..vertices.len(), framebuffer, uniform);
    }

    /// Draw indexed vertices with blending but without depth testing.
    #[inline]
    pub fn draw_indexed<Var, U, C, O>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [O],
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
        B: Fn(C, C) -> C + Sync + Copy,
    {
        self.pipeline.draw_indexed_without_depth_blend(
            vertices,
            indexed,
            framebuffer,
            self.renderer.width,
            self.renderer.height,
            uniform,
            self.blend_mode.0,
        );
    }
}

// ============================================================================
// Draw methods for WithDepth + NoBlend
// ============================================================================
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F>
    BoundPipeline<'a, T, V, F, WithDepth<'a>, NoBlend>
{
    /// Draw all vertices with depth testing but without blending.
    #[inline]
    pub fn draw<Var, U, C>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [C], uniform: &U)
    where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        self.draw_indexed(vertices, 0..vertices.len(), framebuffer, uniform);
    }

    /// Draw indexed vertices with depth testing but without blending.
    #[inline]
    pub fn draw_indexed<Var, U, C>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [C],
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        self.pipeline.draw_indexed(
            vertices,
            indexed,
            self.depth_mode.0,
            framebuffer,
            self.renderer.width,
            self.renderer.height,
            uniform,
        );
    }
}

// ============================================================================
// Draw methods for WithDepth + WithBlend
// ============================================================================
impl<'a, T: Primitive<V::Varying>, V: VertexShader, F, B>
    BoundPipeline<'a, T, V, F, WithDepth<'a>, WithBlend<B>>
{
    /// Draw all vertices with both depth testing and blending.
    #[inline]
    pub fn draw<Var, U, C, O>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [O], uniform: &U)
    where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
        B: Fn(C, C) -> C + Sync + Copy,
    {
        self.draw_indexed(
            vertices,
            0..vertices.len(),
            framebuffer,
            uniform,
            self.blend_mode.0,
        );
    }

    /// Draw indexed vertices with both depth testing and blending.
    #[inline]
    pub fn draw_indexed<Var, U, C, O>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [O],
        uniform: &U,
        blend: impl Fn(C, C) -> C + Sync,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        self.pipeline.draw_indexed_with_depth_blend(
            vertices,
            indexed,
            self.depth_mode.0,
            framebuffer,
            self.renderer.width,
            self.renderer.height,
            uniform,
            blend,
        );
    }
}
