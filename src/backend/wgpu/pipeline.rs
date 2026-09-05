pub struct WgpuRenderPipelineDescriptor<'a, T, V> {
    pub label: Option<&'a str>,

    pub shader: &'a wgpu::ShaderModule,
    pub vertex_entry: Option<&'a str>,
    pub fragment_entry: Option<&'a str>,

    pub bind_group_layouts: &'a [Option<&'a wgpu::BindGroupLayout>],
    pub color_targets: &'a [Option<wgpu::ColorTargetState>],
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub multisample: wgpu::MultisampleState,

    pub strip_index_format: Option<wgpu::IndexFormat>,

    _topology: core::marker::PhantomData<T>,
    _vertex: core::marker::PhantomData<V>,
}

pub struct WgpuRenderPipeline {
    pub(crate) raw: wgpu::RenderPipeline,
}
