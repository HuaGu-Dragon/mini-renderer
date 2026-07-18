use core::marker::PhantomData;

use crate::{
    graphics::{
        FrontFace,
        rasterizer::{LineRasterizer, PointRasterizer, Rasterizer, TriangleRasterizer},
    },
    pipeline::{shader::VertexOutput, varying::Varying},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveTopology<T = ()> {
    _marker: PhantomData<T>,
}

impl PrimitiveTopology {
    pub fn point_list() -> PrimitiveTopology<PointList> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }

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

    pub fn triangle_list() -> PrimitiveTopology<TriangleList> {
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

pub struct TriangleList;
pub struct TriangleStrip;
pub struct TriangleFan;

pub trait Primitive<Var> {
    type Rasterizer: Rasterizer<Var>;
    // FIXME: due to a current limitation of the type system, this implies a 'static lifetime
    // type Primitive<'a, V>
    // where
    //     V: 'a,
    //     Var: 'a;
    // type Primitive<V>;

    fn rasterizer(
        front_face: FrontFace,
        cull_mode: Option<crate::graphics::Face>,
    ) -> Self::Rasterizer {
        Self::Rasterizer::new(front_face, cull_mode)
    }

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    // -> impl Iterator<Item = Self::Primitive<'a, Var>>
    where
        Var: Varying;

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying;

    /// Returns the number of primitives produced by an indexed draw.
    ///
    /// Custom topologies can override this to improve the parallel pipeline's
    /// choice between direct rasterization and spatial binning.
    fn primitive_count(index_count: usize) -> usize {
        index_count
    }
}

impl<Var> Primitive<Var> for PointList {
    type Rasterizer = PointRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.iter().copied()
    }

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        indices.iter().map(|&index| vertexs[index])
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count
    }
}

impl<Var> Primitive<Var> for LineList {
    type Rasterizer = LineRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    where
        Var: Varying,
    {
        let (chunks, _) = vertexs.as_chunks::<2>();
        chunks.iter().copied()
    }

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        let (chunks, _) = indices.as_chunks::<2>();
        chunks.iter().map(|&[i0, i1]| [vertexs[i0], vertexs[i1]])
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count / 2
    }
}

impl<Var> Primitive<Var> for LineStrip {
    type Rasterizer = LineRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.array_windows::<2>().copied()
    }

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        indices
            .array_windows::<2>()
            .map(|&[i0, i1]| [vertexs[i0], vertexs[i1]])
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count.saturating_sub(1)
    }
}

impl<Var> Primitive<Var> for LineLoop {
    type Rasterizer = LineRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
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

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        indices.first().into_iter().flat_map(move |&first| {
            indices
                .array_windows::<2>()
                .map(|&[i0, i1]| [vertexs[i0], vertexs[i1]])
                .chain(core::iter::once([
                    vertexs[indices.last().copied().unwrap_or(first)],
                    vertexs[first],
                ]))
        })
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count
    }
}

impl<Var> Primitive<Var> for TriangleList {
    type Rasterizer = TriangleRasterizer;
    // type Primitive<'a, V>
    //     = &'a [VertexOutput<V>; 3]
    // where
    //     V: 'a,
    //     Var: 'a;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    where
        Var: Varying,
    {
        let (chunks, _) = vertexs.as_chunks::<3>();
        chunks.iter().copied()
    }

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        let (chunks, _) = indices.as_chunks::<3>();
        chunks
            .iter()
            .map(|&[i0, i1, i2]| [vertexs[i0], vertexs[i1], vertexs[i2]])
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count / 3
    }
}

impl<Var> Primitive<Var> for TriangleStrip {
    type Rasterizer = TriangleRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
    where
        Var: Varying,
    {
        vertexs.array_windows::<3>().copied()
    }

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        indices
            .array_windows::<3>()
            .map(|&[i0, i1, i2]| [vertexs[i0], vertexs[i1], vertexs[i2]])
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count.saturating_sub(2)
    }
}

impl<Var> Primitive<Var> for TriangleFan {
    type Rasterizer = TriangleRasterizer;

    fn assemble(
        vertexs: &[VertexOutput<Var>],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>>
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

    fn assemble_indexed<'a>(
        vertexs: &'a [VertexOutput<Var>],
        indices: &'a [usize],
    ) -> impl Iterator<Item = <Self::Rasterizer as Rasterizer<Var>>::Primitive<Var>> + 'a
    where
        Var: Varying,
    {
        indices.first().into_iter().flat_map(move |&first| {
            indices[1..]
                .array_windows::<2>()
                .map(move |&[i1, i2]| [vertexs[first], vertexs[i1], vertexs[i2]])
        })
    }

    fn primitive_count(index_count: usize) -> usize {
        index_count.saturating_sub(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec4;

    #[test]
    fn built_in_topologies_report_assembled_primitive_counts() {
        assert_eq!(<PointList as Primitive<()>>::primitive_count(7), 7);
        assert_eq!(<LineList as Primitive<()>>::primitive_count(7), 3);
        assert_eq!(<LineStrip as Primitive<()>>::primitive_count(7), 6);
        assert_eq!(<LineLoop as Primitive<()>>::primitive_count(7), 7);
        assert_eq!(<TriangleList as Primitive<()>>::primitive_count(7), 2);
        assert_eq!(<TriangleStrip as Primitive<()>>::primitive_count(7), 5);
        assert_eq!(<TriangleFan as Primitive<()>>::primitive_count(7), 5);
    }

    #[test]
    fn triangle_list_assembles_from_index_order() {
        let vertices = [
            VertexOutput {
                position: Vec4::new(0.0, 0.0, 0.0, 1.0),
                varying: 0.0,
            },
            VertexOutput {
                position: Vec4::new(1.0, 0.0, 0.0, 1.0),
                varying: 1.0,
            },
            VertexOutput {
                position: Vec4::new(0.0, 1.0, 0.0, 1.0),
                varying: 2.0,
            },
            VertexOutput {
                position: Vec4::new(1.0, 1.0, 0.0, 1.0),
                varying: 3.0,
            },
        ];
        let indices = [0, 1, 2, 2, 1, 3];

        let primitives = <TriangleList as Primitive<f32>>::assemble_indexed(&vertices, &indices)
            .collect::<Vec<_>>();

        assert_eq!(primitives.len(), 2);
        assert_eq!(primitives[0].map(|vertex| vertex.varying), [0.0, 1.0, 2.0]);
        assert_eq!(primitives[1].map(|vertex| vertex.varying), [2.0, 1.0, 3.0]);
    }
}
