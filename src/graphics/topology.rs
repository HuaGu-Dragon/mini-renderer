use core::marker::PhantomData;

use crate::{
    graphics::{
        FrontFace,
        rasterizer::{LineRasterizer, Rasterizer, TriangleRasterizer},
    },
    pipeline::{shader::VertexOutput, varying::Varying},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveTopology<T = ()> {
    _marker: PhantomData<T>,
}

impl PrimitiveTopology {
    pub fn line_list() -> PrimitiveTopology<LineList> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

    pub fn line_strip() -> PrimitiveTopology<LineStrip> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

    pub fn line_loop() -> PrimitiveTopology<LineLoop> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

    pub fn trangle_list() -> PrimitiveTopology<TrangleList> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

    pub fn triangle_strip() -> PrimitiveTopology<TriangleStrip> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

    pub fn triangle_fan() -> PrimitiveTopology<TriangleFan> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }
}

pub struct PointList;

pub struct LineList;
pub struct LineStrip;
pub struct LineLoop;

pub struct TrangleList;
pub struct TriangleStrip;
pub struct TriangleFan;

pub trait Primitive<Var> {
    type Rasterizer: Rasterizer<Var>;
    // FIXME: due to a current limitation of the type system, this implies a 'static lifetime
    // type Primitive<'a, V>
    // where
    //     V: 'a,
    //     Var: 'a;
    type Primitive<V>;

    fn rasterizer(
        front_face: FrontFace,
        cull_mode: Option<crate::graphics::Face>,
    ) -> Self::Rasterizer {
        Self::Rasterizer::new(front_face, cull_mode)
    }

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    // -> impl Iterator<Item = Self::Primitive<'a, Var>>
    where
        Var: Varying;
}

impl<Var> Primitive<Var> for LineList {
    type Rasterizer = LineRasterizer;
    type Primitive<V> = [VertexOutput<V>; 2];

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        let (chunks, _) = vertexs.as_chunks::<2>();
        chunks.iter().copied()
    }
}

impl<Var> Primitive<Var> for LineStrip {
    type Rasterizer = LineRasterizer;
    type Primitive<V> = [VertexOutput<V>; 2];

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.array_windows::<2>().copied()
    }
}

impl<Var> Primitive<Var> for LineLoop {
    type Rasterizer = LineRasterizer;
    type Primitive<V> = [VertexOutput<V>; 2];

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.first().into_iter().flat_map(move |&first| {
            vertexs
                .array_windows::<2>()
                .copied()
                .chain(core::iter::once([
                    vertexs.last().copied().unwrap_or(first),
                    first,
                ]))
        })
    }
}

impl<Var> Primitive<Var> for TrangleList {
    type Rasterizer = TriangleRasterizer;
    type Primitive<V> = [VertexOutput<V>; 3];
    // type Primitive<'a, V>
    //     = &'a [VertexOutput<V>; 3]
    // where
    //     V: 'a,
    //     Var: 'a;

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        let (chunks, _) = vertexs.as_chunks::<3>();
        chunks.iter().copied()
    }
}

impl<Var> Primitive<Var> for TriangleStrip {
    type Rasterizer = TriangleRasterizer;

    type Primitive<V> = [VertexOutput<V>; 3];

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.array_windows::<3>().copied()
    }
}

impl<Var> Primitive<Var> for TriangleFan {
    type Rasterizer = TriangleRasterizer;

    type Primitive<V> = [VertexOutput<V>; 3];

    fn assemble(vertexs: &[VertexOutput<Var>]) -> impl Iterator<Item = Self::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.first().into_iter().flat_map(move |&first| {
            vertexs[1..]
                .array_windows::<2>()
                .copied()
                .map(move |[v1, v2]| [first, v1, v2])
        })
    }
}
