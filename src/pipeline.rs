use std::fmt::Debug;

use crate::{
    graphics::{color::IntoColor, primitive::PrimitiveAssembler, rasterizer::Rasterizer},
    pipeline::{
        shader::{FragmentShader, VertexInput, VertexShader},
        varying::Varying,
    },
};

pub mod shader;
pub mod varying;

pub struct Pipeline<V, F, R> {
    vertex_shader: V,
    fragment_shader: F,
    rasterizer: R,
    assembler: PrimitiveAssembler,
}

impl<V, F, R> Pipeline<V, F, R> {
    pub fn new(
        vertex_shader: V,
        fragment_shader: F,
        rasterizer: R,
        assembler: PrimitiveAssembler,
    ) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
            rasterizer,
            assembler,
        }
    }

    pub fn draw<Var, C>(
        &mut self,
        vertives: &[VertexInput<V::Vertex, V::Varying>],
        depth_buffer: &mut [f32],
        framebuffer: &mut [C::Output],
        width: usize,
    ) where
        C: IntoColor,
        V: VertexShader<Varying = Var>,
        F: FragmentShader<Varying = Var, Output = C>,
        R: Rasterizer<Var>,
        Var: Varying + Debug,
    {
        let output = vertives
            .iter()
            .enumerate()
            .map(|v| self.vertex_shader.vs_main(v.0, v.1))
            .collect::<Vec<_>>();

        let assembled = self.assembler.assemble(&output[..]);

        let fragments = self.rasterizer.rasterize(assembled);

        fragments.for_each(|f| {
            if f.depth < depth_buffer[f.x + f.y * width] {
                let output = self.fragment_shader.fs_main(&f.varying);
                framebuffer[f.x + f.y * width] = output.into_color();
                depth_buffer[f.x + f.y * width] = f.depth;
            }
        });
    }
}
