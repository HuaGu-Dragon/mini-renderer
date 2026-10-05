use core::marker::PhantomData;

pub struct WgpuPrimitiveState {
    pub strip_index_format: Option<wgpu::IndexFormat>,
    pub front_face: wgpu::FrontFace,
    pub cull_mode: Option<wgpu::Face>,
    pub unclipped_depth: bool,
    pub polygon_mode: wgpu::PolygonMode,
    pub conservative: bool,
}

impl Default for WgpuPrimitiveState {
    fn default() -> Self {
        Self {
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }
}

pub struct WgpuRenderPipelineDescriptor<'a, T, V> {
    pub label: Option<&'a str>,
    pub shader: &'a wgpu::ShaderModule,
    pub vertex_entry: Option<&'a str>,
    pub fragment_entry: Option<&'a str>,
    pub bind_group_layouts: &'a [Option<&'a wgpu::BindGroupLayout>],
    pub color_targets: &'a [Option<wgpu::ColorTargetState>],
    pub primitive: WgpuPrimitiveState,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub multisample: wgpu::MultisampleState,
    _topology: PhantomData<T>,
    _vertex: PhantomData<V>,
}

impl<'a, T, V> WgpuRenderPipelineDescriptor<'a, T, V> {
    pub fn new(
        shader: &'a wgpu::ShaderModule,
        color_targets: &'a [Option<wgpu::ColorTargetState>],
    ) -> Self {
        Self {
            label: None,
            shader,
            vertex_entry: Some("vs_main"),
            fragment_entry: Some("fs_main"),
            bind_group_layouts: &[],
            color_targets,
            primitive: WgpuPrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            _topology: PhantomData,
            _vertex: PhantomData,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self {
        self.vertex_entry = Some(entry);
        self
    }

    pub fn with_fragment_entry(mut self, entry: &'a str) -> Self {
        self.fragment_entry = Some(entry);
        self
    }

    pub fn with_bind_group_layouts(
        mut self,
        layouts: &'a [Option<&'a wgpu::BindGroupLayout>],
    ) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    pub fn with_primitive(mut self, primitive: WgpuPrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    pub fn with_strip_index_format(mut self, format: wgpu::IndexFormat) -> Self {
        self.primitive.strip_index_format = Some(format);
        self
    }

    pub fn with_front_face(mut self, front_face: wgpu::FrontFace) -> Self {
        self.primitive.front_face = front_face;
        self
    }

    pub fn with_cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.primitive.cull_mode = cull_mode;
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    pub fn with_multisample(mut self, multisample: wgpu::MultisampleState) -> Self {
        self.multisample = multisample;
        self
    }
}

pub struct WgpuRenderPipeline {
    pub(crate) raw: wgpu::RenderPipeline,
}

impl WgpuRenderPipeline {
    pub fn raw(&self) -> &wgpu::RenderPipeline {
        &self.raw
    }
}
