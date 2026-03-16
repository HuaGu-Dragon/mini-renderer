use std::{fmt::Debug, marker::PhantomData};

use crate::{
    graphics::{rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        shader::{FragmentShader, VertexInput, VertexShader},
        varying::Varying,
    },
};

pub mod shader;
pub mod varying;

pub struct Pipeline<T, R, V, F> {
    _marker: PhantomData<T>,
    rasterizer: R,
    vertex_shader: V,
    fragment_shader: F,
}

impl<T, R, V, F> Pipeline<T, R, V, F> {
    pub fn new(rasterizer: R, vertex_shader: V, fragment_shader: F) -> Self {
        Self {
            _marker: PhantomData,
            rasterizer,
            vertex_shader,
            fragment_shader,
        }
    }

    pub fn draw<Var, C>(
        &mut self,
        vertives: &[VertexInput<V::Vertex, V::Varying>],
        depth_buffer: &mut [f32],
        framebuffer: &mut [C],
        width: usize,
        height: usize,
    ) where
        // FIXME: due to a current limitation of the type system, this implies a 'static lifetime
        // T: for<'a> Primitive<
        //         Var,
        //         Rasterizer = R,
        //         Primitive<'a, Var> = <R as Rasterizer<Var>>::Primitive<'a, Var>,
        //     >,
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var>,
        V: VertexShader<Varying = Var>,
        F: FragmentShader<Varying = Var, Output = C>,
        Var: Varying + Debug,
    {
        self.vertex_shader.update();
        self.fragment_shader.update();

        let output = vertives
            .iter()
            .enumerate()
            .map(|v| self.vertex_shader.vs_main(v.0, v.1))
            .collect::<Vec<_>>();

        let assembled = T::assemble(&output[..]);

        let fragments = self.rasterizer.rasterize(assembled, width, height);

        fragments.for_each(|f| {
            if f.depth < depth_buffer[f.x + f.y * width] {
                let Some(output) = self.fragment_shader.fs_main(&f.varying) else {
                    return;
                };
                framebuffer[f.x + f.y * width] = output;
                depth_buffer[f.x + f.y * width] = f.depth;
            }
        });
    }

    pub fn draw_indexed<Var, C>(
        &mut self,
        vertives: &[VertexInput<V::Vertex, V::Varying>],
        indexed: impl Iterator<Item = usize>,
        depth_buffer: &mut [f32],
        framebuffer: &mut [C],
        width: usize,
        height: usize,
    ) where
        // FIXME: due to a current limitation of the type system, this implies a 'static lifetime
        // T: for<'a> Primitive<
        //         Var,
        //         Rasterizer = R,
        //         Primitive<'a, Var> = <R as Rasterizer<Var>>::Primitive<'a, Var>,
        //     >,
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var>,
        V: VertexShader<Varying = Var>,
        F: FragmentShader<Varying = Var, Output = C>,
        Var: Varying + Debug,
    {
        self.vertex_shader.update();
        self.fragment_shader.update();

        let output = indexed
            .map(|idx| self.vertex_shader.vs_main(idx, &vertives[idx]))
            .collect::<Vec<_>>();

        let assembled = T::assemble(&output[..]);

        let fragments = self.rasterizer.rasterize(assembled, width, height);

        fragments.for_each(|f| {
            if f.depth < depth_buffer[f.x + f.y * width] {
                let Some(output) = self.fragment_shader.fs_main(&f.varying) else {
                    return;
                };
                framebuffer[f.x + f.y * width] = output;
                depth_buffer[f.x + f.y * width] = f.depth;
            }
        });
    }
}
