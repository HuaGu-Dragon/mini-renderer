# mini-renderer

A lightweight software rasterizer written in Rust with a focus on type-safe rendering pipelines and minimal dependencies.

## Features

### Core Rendering
- **Software Rasterization** - Triangle rasterization with per-pixel processing
- **Depth Testing** - Z-buffer for correct depth ordering
- **Color Blending** - Flexible alpha blending with `From` trait
- **Texture Sampling** - 2D texture mapping with bilinear filtering
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
use mini_renderer::renderer::*;
use mini_renderer::pipeline::shader::*;

// Define your vertex and fragment shaders
struct MyVertex {
    position: (f32, f32),
    color: (f32, f32, f32),
}

struct MyVarying {
    color: (f32, f32, f32),
}

struct MyVertexShader;
impl VertexShader for MyVertexShader {
    type Vertex = MyVertex;
    type Uniform = ();

    fn vs_main(&self, _idx: usize, vertex: &MyVertex, _uniform: &()) -> VertexOutput<MyVarying> {
        VertexOutput {
            position: vertex.position,
            varying: MyVarying { color: vertex.color },
        }
    }
}

struct MyFragmentShader;
impl FragmentShader for MyFragmentShader {
    type Varying = MyVarying;
    type Uniform = ();
    type Output = u32;  // RGBA8888 color

    fn fs_main(&self, varying: &MyVarying, _uniform: &()) -> Option<Self::Output> {
        let (r, g, b) = varying.color;
        Some(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }
}

// Create renderer and pipeline
let renderer = Renderer::new(800, 600);
let mut pipeline = create_render_pipeline(
    MyVertexShader,
    MyFragmentShader,
    PrimitiveState::default(),
);

// Render with depth and blending
renderer
    .begin_render_pass()
    .set_pipeline(&mut pipeline)
    .with_depth(&mut depth_buffer)
    .with_blend()
    .draw_indexed(&vertices, indices, &mut framebuffer, &uniform);
```

## Examples

### 1. Triangle (`examples/triangle.rs`)
Simple triangle rendering with depth buffer and animation.

```bash
cargo run --example triangle
```

### 2. Texture 2D (`examples/texture2D.rs`)
Textured quad with bilinear filtering.

```bash
cargo run --example texture2D
```

### 3. 3D Model (`examples/model.rs`)
OBJ model loading and rendering with camera controls.

```bash
cargo run --example model
```

Features:
- Model loading from OBJ files
- Multi-texture support
- Camera with WASD + QE controls
- Depth testing

### 4. Egui Integration (`examples/egui.rs`)
Integration with Egui for immediate-mode UI rendering.

```bash
cargo run --example egui
```

Features:
- Batched mesh rendering
- Per-batch texture switching
- Clip rectangle support
- Color blending for transparency

### 5. Sandbox (`examples/sandbox.rs`)
Experimental rendering sandbox.

## Architecture

### Module Structure

```
src/
├── lib.rs              # Library entry point
├── renderer.rs         # Rendering pass and pipeline binding
├── pipeline/
│   ├── mod.rs         # Pipeline definition
│   ├── shader.rs      # Vertex/Fragment shader traits
│   └── varying.rs     # Varying interpolation trait
├── graphics/
│   ├── mod.rs
│   ├── primitive.rs   # Primitive assembly
│   └── rasterizer.rs  # Triangle rasterization
├── math.rs            # Math utilities (AABB, barycentric coordinates)
└── color.rs           # Color format abstraction
```

### Key Types

#### `Renderer`
Main rendering interface:
```rust
pub struct Renderer {
    pub width: usize,
    pub height: usize,
}

impl Renderer {
    pub fn begin_render_pass(&self) -> RenderPass;
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
.with_blend()

// Available in appropriate states
.draw(vertices, framebuffer, uniform)
.draw_indexed(vertices, indices, framebuffer, uniform)
```

#### `Pipeline<T, R, V, F>`
Low-level rendering pipeline:
- `T`: Primitive type
- `R`: Rasterizer implementation
- `V`: Vertex shader
- `F`: Fragment shader

#### Shader Traits

**VertexShader**
```rust
pub trait VertexShader {
    type Vertex;
    type Uniform: ?Sized;

    fn vs_main(&self, index: usize, vertex: &Self::Vertex, uniform: &Self::Uniform) 
        -> VertexOutput<Self::Varying>;
}
```

**FragmentShader**
```rust
pub trait FragmentShader {
    type Varying: Varying;
    type Uniform: ?Sized;
    type Output;

    fn fs_main(&self, varying: &Self::Varying, uniform: &Self::Uniform) 
        -> Option<Self::Output>;

    fn blend(&self, src: Self::Output, dst: Self::Output) -> Self::Output {
        src  // Default: source over
    }
}
```

#### `Varying`
Custom interpolation for vertex attributes:
```rust
pub trait Varying {
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self;
}
```

## Design Patterns

### Type-Safe Pipeline State

The renderer uses Rust's type system to prevent invalid state combinations:

```rust
// Compile error: can't call with_blend() twice
pipeline.with_blend().with_blend().draw_indexed(...);

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
    .with_blend()                       // Optional
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
- Vertices are stored in interleaved format for cache efficiency
- Depth buffer uses `f32` for maximum precision
- Framebuffer format is generic (`T: Send`)

## Dependencies

### Runtime
- **rayon** (1.11.0) - Parallel rasterization

### Optional
- **glam** (0.32.0) - Math library (enabled by default)

### Development
- **egui** (0.33.3) - UI framework for example
- **winit** (0.30.12) - Window management
- **softbuffer** - Software framebuffer
- **image** (0.25.9) - Image loading
- **tobj** (4.0.3) - OBJ model loading

## Limitations

### What's Not Implemented
- GPU acceleration (pure software rasterization)
- Advanced lighting models (Blinn-Phong, PBR)
- Shadow mapping
- MSAA or post-processing effects
- Normal mapping or displacement mapping
- Deferred rendering
- Compute shaders

### Design Constraints
- Single-threaded shader execution (per-vertex/fragment)
- Limited Uniform size (data must fit in memory)
- No support for hardware textures
- 2D texture sampling only (no cubemaps)

## Contributing

The codebase is organized for clarity and extensibility:

1. **New Primitive Type?** Implement `Primitive` trait in `graphics/topology.rs`
2. **New Rasterizer?** Implement `Rasterizer` trait in `graphics/rasterizer.rs`
3. **Custom Shaders?** Implement `VertexShader` and `FragmentShader` traits

## Future Improvements

- [ ] SIMD optimizations for rasterization
- [ ] Advanced interpolation methods (perspective-correct)
- [ ] Texture compression support
- [ ] Material system with multiple render passes

## License

MIT

## Project Structure Overview

```
mini-renderer/
├── Cargo.toml             # Project manifest
├── README.md              # This file
├── src/                   # Library code
│   ├── lib.rs             # Module exports
│   ├── renderer.rs        # High-level rendering API
│   ├── pipeline.rs        # Pipeline implementation
│   ├── graphics.rs        # Graphics module
│   └── math.rs            # Math utilities
├── examples/              # Runnable examples
│   ├── triangle.rs        # Basic triangle
│   ├── texture2D.rs       # Textured quad
│   ├── model.rs           # 3D model rendering
│   ├── egui.rs            # UI integration
│   └── ...
└── assets/                # Example assets (models, images)
```

## Getting Started

### Prerequisites
- Rust
- Cargo

### Installation

```bash
git clone https://github.com/HuaGu-Dragon/mini-renderer.git
cd mini-renderer
```

### Run Examples

```bash
# Basic triangle
cargo run --example triangle -r

# 3D model with texture
cargo run --example model -r

# UI rendering with Egui
cargo run --example egui -r
```

### Use in Your Project

Add to `Cargo.toml`:
```toml
[dependencies]
mini-renderer = { git = "https://github.com/HuaGu-Dragon/mini-renderer.git"}
```

## Technical Details

### Rendering Pipeline Flow

```
1. Vertex Processing
   Input: Vertices + Uniform
   Output: Transformed vertices with Varying data

2. Primitive Assembly
   Input: Transformed vertices + Index buffer
   Output: Primitives (triangles)

3. Rasterization
   Input: Primitives
   Output: Fragments (per-pixel data)

4. Fragment Processing
   Input: Fragments + Varying
   Output: Final color (with optional blending)

5. Framebuffer Write
   Input: Final colors
   Output: Displayed image
```

### Interpolation Strategy

Barycentric coordinate interpolation for smooth gradients across triangles:
```
interpolated = w0 * v0 + w1 * v1 + w2 * v2
```

Where `w0, w1, w2` are barycentric weights (summing to 1.0).

### Depth Testing

Simple depth test: write fragment if `z < existing_z`

```rust
if f.depth < db_chunk[local_idx] {
    // Write fragment
    fb_chunk[local_idx] = output;
    db_chunk[local_idx] = f.depth;
}
```

### Color Blending

Standard "over" blending with customizable implementation:
```rust
result = src + dst * (1 - src.alpha)
```

## Contact & Support

For issues, questions, or suggestions, please open an issue on GitHub or contact the maintainers.

Happy rendering! 🎨
