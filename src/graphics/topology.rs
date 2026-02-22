#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    // PointList,

    // LineList,

    // LineStrip,
    TriangleList,
    // TriangleStrip,

    // TriangleFan,
}

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
