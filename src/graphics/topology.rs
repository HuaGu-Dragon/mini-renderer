use core::marker::PhantomData;

use crate::{
    graphics::{
        FrontFace,
        rasterizer::{Rasterizer, TriangleRasterizer},
    },
    pipeline::{shader::VertexOutput, varying::Varying},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveTopology<T = ()> {
    _marker: PhantomData<T>,
}

impl PrimitiveTopology {
    pub fn trangle_list() -> PrimitiveTopology<TrangleList> {
        PrimitiveTopology {
            _marker: PhantomData,
        }
    }
}

pub struct TrangleList;

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

// impl<Var> Primitive<Var> for TrangleList {
//     type Rasterizer = TriangleRasterizer;
//     type Primitive = (&VertexOutput<Var>, &VertexOutput<Var>, &VertexOutput<Var>);
// }
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum PrimitiveTopology {
//     PointList,

//     LineList,

//     LineStrip,
//     TriangleList,
//     TriangleStrip,

//     TriangleFan,
// }

// impl PrimitiveTopology {
//     pub fn min_vertices(&self) -> usize {
//         match self {
//             PrimitiveTopology::PointList => 1,
//             PrimitiveTopology::LineList | PrimitiveTopology::LineStrip => 2,
//             PrimitiveTopology::TriangleList
//             | PrimitiveTopology::TriangleStrip
//             | PrimitiveTopology::TriangleFan => 3,
//         }
//     }

//     pub fn primitive_count(&self, vertex_count: usize) -> usize {
//         if vertex_count < self.min_vertices() {
//             return 0;
//         }

//         match self {
//             PrimitiveTopology::PointList => vertex_count,
//             PrimitiveTopology::LineList => vertex_count / 2,
//             PrimitiveTopology::LineStrip => vertex_count - 1,
//             PrimitiveTopology::TriangleList => vertex_count / 3,
//             PrimitiveTopology::TriangleStrip | PrimitiveTopology::TriangleFan => {
//                 vertex_count.saturating_sub(2)
//             }
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_primitive_count() {
//         assert_eq!(PrimitiveTopology::PointList.primitive_count(5), 5);
//         assert_eq!(PrimitiveTopology::LineList.primitive_count(6), 3);
//         assert_eq!(PrimitiveTopology::LineStrip.primitive_count(5), 4);
//         assert_eq!(PrimitiveTopology::TriangleList.primitive_count(9), 3);
//         assert_eq!(PrimitiveTopology::TriangleStrip.primitive_count(5), 3);
//         assert_eq!(PrimitiveTopology::TriangleFan.primitive_count(5), 3);
//     }

//     #[test]
//     fn test_min_vertices() {
//         assert_eq!(PrimitiveTopology::PointList.min_vertices(), 1);
//         assert_eq!(PrimitiveTopology::LineList.min_vertices(), 2);
//         assert_eq!(PrimitiveTopology::TriangleList.min_vertices(), 3);
//     }
// }
