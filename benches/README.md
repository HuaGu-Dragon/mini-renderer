# Rasterizer benchmarks

The `rasterizer` benchmark renders deterministic triangle grids into a 512×512 framebuffer. It covers both color-only rendering and rendering with a depth buffer at two primitive counts. A separate indexed-grid case reuses vertices so changes to indexed vertex processing are measurable.

Run the default, Rayon-enabled pipeline with:

```console
cargo bench --bench rasterizer
```

Run the serial pipeline with:

```console
cargo bench --no-default-features --features std --bench rasterizer
```

Criterion prepares fresh color and depth buffers outside the measured routine. The reported time therefore covers vertex shading, primitive assembly, rasterization, fragment shading, and buffer writes without including buffer allocation or clearing.
