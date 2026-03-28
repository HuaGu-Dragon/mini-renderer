use std::marker::PhantomData;

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
) -> Pipeline<T, T::Rasterizer, VS, FS>
where
    T: Primitive<VS::Varying>,
    VS: VertexShader,
    FS: FragmentShader<Varying = VS::Varying>,
{
    let rasterizer = T::rasterizer(primitive.front_face, primitive.cull_mode);

    Pipeline::new(rasterizer, vertex_shader, fragment_shader)
}

pub struct Renderer {
    pub width: usize,
    pub height: usize,
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
pub struct WithBlend;

/// Represents a pipeline bound to a render pass, with compile-time-known depth and blend modes.
///
/// Type parameters:
/// - `T`: Primitive type
/// - `R`: Rasterizer type
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
pub struct BoundPipeline<'a, T, R, V: VertexShader, F, D = NoDepth, B = NoBlend> {
    render: &'a Renderer,
    pipeline: &'a mut Pipeline<T, R, V, F>,
    depth_mode: D,
    _blend_mode: PhantomData<B>,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn begin_render_pass(&self) -> RenderPass<'_> {
        RenderPass { render: self }
    }
}

impl<'pass> RenderPass<'pass> {
    pub fn set_pipeline<'a, T, R, V: VertexShader, F>(
        &'a self,
        pipeline: &'a mut Pipeline<T, R, V, F>,
    ) -> BoundPipeline<'a, T, R, V, F, NoDepth, NoBlend> {
        BoundPipeline {
            render: self.render,
            pipeline,
            depth_mode: NoDepth,
            _blend_mode: PhantomData,
        }
    }
}

// Methods to transition from NoDepth state
impl<'a, T, R, V: VertexShader, F, B> BoundPipeline<'a, T, R, V, F, NoDepth, B> {
    /// Enable depth testing with the provided depth buffer.
    pub fn with_depth(
        self,
        depth_buffer: &'a mut [f32],
    ) -> BoundPipeline<'a, T, R, V, F, WithDepth<'a>, B> {
        BoundPipeline {
            render: self.render,
            pipeline: self.pipeline,
            depth_mode: WithDepth(depth_buffer),
            _blend_mode: PhantomData,
        }
    }
}

// Methods to transition from NoBlend state
impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, NoDepth, NoBlend> {
    /// Enable blending (requires bidirectional From/Into conversion).
    pub fn with_blend(self) -> BoundPipeline<'a, T, R, V, F, NoDepth, WithBlend> {
        BoundPipeline {
            render: self.render,
            pipeline: self.pipeline,
            depth_mode: NoDepth,
            _blend_mode: PhantomData,
        }
    }
}

impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, WithDepth<'a>, NoBlend> {
    /// Enable blending (requires bidirectional From/Into conversion).
    pub fn with_blend(self) -> BoundPipeline<'a, T, R, V, F, WithDepth<'a>, WithBlend> {
        BoundPipeline {
            render: self.render,
            pipeline: self.pipeline,
            depth_mode: self.depth_mode,
            _blend_mode: PhantomData,
        }
    }
}

// ============================================================================
// Draw methods for NoDepth + NoBlend
// ============================================================================
impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, NoDepth, NoBlend> {
    /// Draw all vertices without depth testing or blending.
    #[inline]
    pub fn draw<Var, U, C>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [C], uniform: &U)
    where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
    {
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
    {
        self.pipeline.draw_indexed_without_depth(
            vertices,
            indexed,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }
}

// ============================================================================
// Draw methods for NoDepth + WithBlend
// ============================================================================
impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, NoDepth, WithBlend> {
    /// Draw all vertices with blending but without depth testing.
    #[inline]
    pub fn draw<Var, U, C, O>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [O], uniform: &U)
    where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
    {
        self.pipeline.draw_indexed_without_depth_blend(
            vertices,
            indexed,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }
}

// ============================================================================
// Draw methods for WithDepth + NoBlend
// ============================================================================
impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, WithDepth<'a>, NoBlend> {
    /// Draw all vertices with depth testing but without blending.
    #[inline]
    pub fn draw<Var, U, C>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [C], uniform: &U)
    where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
    {
        self.pipeline.draw_indexed(
            vertices,
            indexed,
            self.depth_mode.0,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }
}

// ============================================================================
// Draw methods for WithDepth + WithBlend
// ============================================================================
impl<'a, T, R, V: VertexShader, F> BoundPipeline<'a, T, R, V, F, WithDepth<'a>, WithBlend> {
    /// Draw all vertices with both depth testing and blending.
    #[inline]
    pub fn draw<Var, U, C, O>(&mut self, vertices: &[V::Vertex], framebuffer: &mut [O], uniform: &U)
    where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
    {
        self.draw_indexed(vertices, 0..vertices.len(), framebuffer, uniform);
    }

    /// Draw indexed vertices with both depth testing and blending.
    #[inline]
    pub fn draw_indexed<Var, U, C, O>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [O],
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
    {
        self.pipeline.draw_indexed_with_depth_blend(
            vertices,
            indexed,
            self.depth_mode.0,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }
}
