# mini-renderer

A lightweight software rasterizer written in Rust with a focus on type-safe rendering pipelines and minimal dependencies.

## Features

### Core Rendering
- **Software Rasterization** - Triangle rasterization with per-pixel processing
- **Depth Testing** - Z-buffer for correct depth ordering
- **Color Blending** - Custom blending through `FragmentShader::blend`
- **Texture Sampling Examples** - 2D texture mapping with nearest-neighbor sampling
- **Perspective-Correct Interpolation** - Correct varying interpolation in screen space
- **Multi-threaded** - Parallel rasterization using Rayon

### Pipeline Architecture
- **Type-Safe Pipeline** - Compile-time vertex/fragment shader validation
- **Generic Rasterizer** - Support for different primitive types (triangles, lines, etc.)
- **Flexible Shaders** - Trait-based vertex and fragment shader system
- **Varying Interpolation** - Customizable per-vertex attribute interpolation

### Rendering Modes (Builder Pattern)
- **Flexible State Management** - Compose rendering features at compile time:
  - `.with_depth(depth_buffer)` - Enable depth testing
  - `.with_blend()` - Enable color blending
  - Combine freely: `.with_depth(...).with_blend()`
- **Type-Safe Composition** - Invalid state combinations fail at compile time

## Quick Start

### Basic Triangle Example

```rust
use mini_renderer::{
    graphics::{Face, primitive::PrimitiveState},
    math::Vec4,
    pipeline::shader::{FragmentShader, VertexOutput, VertexShader},
    renderer::{Renderer, create_render_pipeline},
};

struct MyVertex {
    position: (f32, f32),
    color: (f32, f32, f32),
}

#[derive(Clone, Copy, mini_renderer::Varying)]
struct MyVarying {
    color: (f32, f32, f32),
}

struct MyVertexShader;

impl VertexShader for MyVertexShader {
    type Vertex = MyVertex;
    type Varying = MyVarying;
    type Uniform = ();

    fn vs_main(
        &self,
        _index: usize,
        vertex: &Self::Vertex,
        _uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        VertexOutput {
            position: Vec4::new(vertex.position.0, vertex.position.1, 0.0, 1.0),
            varying: MyVarying { color: vertex.color },
        }
    }
}

struct MyFragmentShader;

impl FragmentShader for MyFragmentShader {
    type Varying = MyVarying;
    type Output = u32;
    type Uniform = ();

    fn fs_main(
        &self,
        varying: &Self::Varying,
        _uniform: &Self::Uniform,
    ) -> Option<Self::Output> {
        let (r, g, b) = varying.color;
        let r = (r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (b.clamp(0.0, 1.0) * 255.0) as u32;
        Some((r << 24) | (g << 16) | (b << 8) | 0xff)
    }
}

let vertices = [
    MyVertex { position: (-0.5, -0.5), color: (1.0, 0.0, 0.0) },
    MyVertex { position: (0.5, -0.5), color: (0.0, 1.0, 0.0) },
    MyVertex { position: (0.0, 0.5), color: (0.0, 0.0, 1.0) },
];
let indices = [0usize, 1, 2];
let mut framebuffer = vec![0u32; 800 * 600];
let mut depth_buffer = vec![1.0; 800 * 600];

let renderer = Renderer::new(800, 600);
let mut pipeline = create_render_pipeline(
    MyVertexShader,
    MyFragmentShader,
    PrimitiveState::default().with_cull_mode(Face::Back),
);

renderer
    .begin_render_pass()
    .set_pipeline(&mut pipeline)
    .with_depth(&mut depth_buffer)
    .draw_indexed(&vertices, indices.into_iter(), &mut framebuffer, &());
```

`PrimitiveState::default()` uses triangle-list topology, counter-clockwise front faces, and no culling. Use `PrimitiveState::new(PrimitiveTopology::...)` to select another topology.

## Toolchain and feature sets

mini-renderer requires Rust 1.94 or newer. The default feature set enables the standard library, `glam`, the `Varying` derive macro, and Rayon-based parallel rendering.

For a serial `no_std` build with `libm`, run:

```console
cargo check --no-default-features --features libm
```

## Architecture

### Module Structure

```
src/
├── lib.rs             # Library entry point
├── renderer.rs        # Rendering pass and pipeline binding
├── pipeline/
│   ├── mod.rs         # Pipeline definition
│   ├── shader.rs      # Vertex/Fragment shader traits
│   └── varying.rs     # Varying interpolation trait
├── graphics/
│   ├── mod.rs
│   ├── primitive.rs   # Primitive pipeline state
│   ├── rasterizer.rs  # Point, line, and triangle rasterization
│   └── topology.rs    # Primitive assembly and topology markers
└── math.rs            # Vector types and math utilities
```

### Key Types

#### `Renderer`
Main rendering interface:
```rust
pub struct Renderer {
    width: usize,
    height: usize,
}

impl Renderer {
    pub fn begin_render_pass(&self) -> RenderPass<'_>;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
}
```

#### `BoundPipeline<D, B>`
Type-safe pipeline state with depth and blend modes:
- `D`: Depth mode (`NoDepth` or `WithDepth`)
- `B`: Blend mode (`NoBlend` or `WithBlend`)

Methods available depend on state:
```rust
// Only on NoDepth
.with_depth(depth_buffer)

// Only on NoBlend  
.with_blend(blend)

// Available in appropriate states
.draw(vertices, framebuffer, uniform)
.draw_indexed(vertices, indices, framebuffer, uniform)
```

#### `Pipeline<T, V, F>`
Low-level rendering pipeline:
- `T`: Primitive type
- `V`: Vertex shader
- `F`: Fragment shader

#### Shader Traits

**VertexShader**
```rust
pub trait VertexShader {
    type Vertex;
    type Varying;
    type Uniform;

    fn vs_main(
        &self,
        index: usize,
        vertex: &Self::Vertex,
        uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying>;
}
```

**FragmentShader**
```rust
pub trait FragmentShader {
    type Varying;
    type Output: Copy;
    type Uniform;

    fn fs_main(
        &self,
        varying: &Self::Varying,
        uniform: &Self::Uniform,
    ) -> Option<Self::Output>;
}
```

#### `Varying`
Custom interpolation for vertex attributes:
```rust
pub trait Varying: Sized + Copy {
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self;
}
```

## Design Patterns

### Type-Safe Pipeline State

The renderer uses Rust's type system to prevent invalid state combinations:

```rust
// Compile error: can't call with_blend() twice
pipeline.with_blend(blend).with_blend(blend).draw_indexed(...);

// Compile error: can't draw without fragment shader state
pipeline.draw_indexed(...);  // Missing method in initial state
```

### Builder Pattern for Rendering

Fluent API for composing rendering operations:

```rust
renderer
    .begin_render_pass()
    .set_pipeline(&mut pipeline)
    .with_depth(&mut depth_buffer)      // Optional
    .with_blend(blend)                  // Optional
    .draw_indexed(vertices, indices, framebuffer, uniform);
```

### Trait-Based Customization

Users define custom behavior via traits:
- `VertexShader` - Vertex transformation and varying output
- `FragmentShader` - Fragment color and blending
- `Varying` - Attribute interpolation strategy

## Performance Considerations

### Multi-threaded Rasterization
The rasterizer uses Rayon to parallelize per-tile processing. Work is distributed across CPU cores for better performance on large framebuffers.

### Rendering Modes Overhead
The `with_depth()` and `with_blend()` methods use Rust's type system with zero runtime cost (compile-time specialization via monomorphization).

### Memory Layout
- Vertex layout is defined by the user's `VertexShader::Vertex` type
- Pipeline-owned vertex and index caches are reused between draw calls
- Depth buffers use `f32`
- Framebuffer element types are generic

## Dependencies

### Runtime
- **rayon** (1.11.0) - Parallel rasterization

### Optional
- **glam** (0.32.0) - Math library (enabled by default)

## Limitations

### What's Not Implemented
- GPU acceleration (pure software rasterization)
- Compute shaders

### Design Constraints
- Shaders execute on the CPU; the default `rayon` feature parallelizes their work
- Vertex and fragment shaders currently share one uniform type per draw
- No GPU acceleration or hardware texture sampling
- Texture sampling is implemented by examples rather than a core texture abstraction

## Contributing

The codebase is organized for clarity and extensibility:

1. **New Primitive Type?** Implement `Primitive` trait in `graphics/topology.rs`
2. **New Rasterizer?** Implement `Rasterizer` trait in `graphics/rasterizer.rs`
3. **Custom Shaders?** Implement `VertexShader` and `FragmentShader` traits

## Future Improvements

- [ ] SIMD optimizations for rasterization
- [ ] Homogeneous clipping before perspective division
- [ ] Texture compression support
- [ ] Material system with multiple render passes

## License

MIT

## Contact & Support

For issues, questions, or suggestions, please open an issue on GitHub or contact the maintainers.

Happy rendering! 🎨
