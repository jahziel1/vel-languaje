use web_sys::HtmlCanvasElement;

pub struct GpuState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub rect_pipeline: wgpu::RenderPipeline,
    pub overlay_pipeline: wgpu::RenderPipeline,
    pub camera_buf: wgpu::Buffer,
    pub instance_buf: wgpu::Buffer,
    pub camera_bg: wgpu::BindGroup,
    pub overlay_sampler: wgpu::Sampler,
    pub overlay_tex: Option<wgpu::Texture>,
    pub overlay_tex_size: (u32, u32),
}

pub const MAX_RECTS: usize = 4096;
// 16 f32 per instance: xy(2) + wh(2) + fill(4) + extra(4:radius,bw,0,0) + bc(4)
pub const RECT_FLOATS: usize = 16;
pub const RECT_STRIDE: u64 = (RECT_FLOATS * 4) as u64;

const RECT_WGSL: &str = r#"
struct Cam { inv_vp: vec2<f32>, _pad: vec2<f32> }
@group(0) @binding(0) var<uniform> cam: Cam;

struct Vs {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv:     vec2<f32>,
    @location(1) sz:     vec2<f32>,
    @location(2) fill:   vec4<f32>,
    @location(3) radius: f32,
    @location(4) bw:     f32,
    @location(5) bc:     vec4<f32>,
}

@vertex fn vs(
    @builtin(vertex_index) vi: u32,
    @location(0) xy:    vec2<f32>,
    @location(1) wh:    vec2<f32>,
    @location(2) fill:  vec4<f32>,
    @location(3) extra: vec4<f32>,
    @location(4) bc:    vec4<f32>,
) -> Vs {
    const QUAD = array(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(1.,1.));
    let uv = QUAD[vi];
    let px = xy + uv * wh;
    var o: Vs;
    o.pos = vec4(px * cam.inv_vp * 2. - 1., 0., 1.);
    o.pos.y = -o.pos.y;
    o.uv = uv; o.sz = wh; o.fill = fill;
    o.radius = extra.x; o.bw = extra.y; o.bc = bc;
    return o;
}

fn sdf(uv: vec2<f32>, sz: vec2<f32>, r: f32) -> f32 {
    let q = abs(uv * sz - sz * .5) - (sz * .5 - r);
    return length(max(q, vec2(0.))) + min(max(q.x, q.y), 0.) - r;
}

@fragment fn fs(v: Vs) -> @location(0) vec4<f32> {
    let r = clamp(v.radius, 0., min(v.sz.x, v.sz.y) * .5);
    let d = sdf(v.uv, v.sz, r);
    let a = clamp(-d, 0., 1.);
    var col = vec4(v.fill.rgb * v.fill.a, v.fill.a) * a;
    if v.bw > 0. {
        let bd = clamp(v.bw - abs(d), 0., 1.);
        col = mix(col, vec4(v.bc.rgb * v.bc.a, v.bc.a), bd);
    }
    return col;
}
"#;

const OVERLAY_WGSL: &str = r#"
@group(0) @binding(0) var s: sampler;
@group(0) @binding(1) var t: texture_2d<f32>;

struct Vs { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex fn vs(@builtin(vertex_index) vi: u32) -> Vs {
    const POS = array(vec2(-1.,-1.),vec2(1.,-1.),vec2(-1.,1.),vec2(1.,1.));
    const UV  = array(vec2(0.,1.),vec2(1.,1.),vec2(0.,0.),vec2(1.,0.));
    return Vs(vec4(POS[vi],0.,1.), UV[vi]);
}

@fragment fn fs(v: Vs) -> @location(0) vec4<f32> {
    return textureSample(t, s, v.uv);
}
"#;

fn premul_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation:  wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation:  wgpu::BlendOperation::Add,
        },
    }
}

fn alpha_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation:  wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation:  wgpu::BlendOperation::Add,
        },
    }
}

impl GpuState {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // SAFETY: canvas lives for the lifetime of the page.
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| format!("surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("no WebGPU adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("vel"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("device: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: 1,
                height: 1,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (MAX_RECTS * RECT_FLOATS * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let rect_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect"),
            source: wgpu::ShaderSource::Wgsl(RECT_WGSL.into()),
        });
        let cam_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&cam_bgl],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &rect_mod,
                entry_point: Some("vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: RECT_STRIDE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute { shader_location: 0, offset: 0,  format: wgpu::VertexFormat::Float32x2 },
                        wgpu::VertexAttribute { shader_location: 1, offset: 8,  format: wgpu::VertexFormat::Float32x2 },
                        wgpu::VertexAttribute { shader_location: 2, offset: 16, format: wgpu::VertexFormat::Float32x4 },
                        wgpu::VertexAttribute { shader_location: 3, offset: 32, format: wgpu::VertexFormat::Float32x4 },
                        wgpu::VertexAttribute { shader_location: 4, offset: 48, format: wgpu::VertexFormat::Float32x4 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_mod,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(premul_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let overlay_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("overlay"),
            source: wgpu::ShaderSource::Wgsl(OVERLAY_WGSL.into()),
        });
        let overlay_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&overlay_bgl],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &overlay_mod,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &overlay_mod,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(alpha_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &cam_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        let overlay_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_format,
            rect_pipeline,
            overlay_pipeline,
            camera_buf,
            instance_buf,
            camera_bg,
            overlay_sampler,
            overlay_tex: None,
            overlay_tex_size: (0, 0),
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.surface_format,
                width: w,
                height: h,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
        let cam = [1.0f32 / w as f32, 1.0 / h as f32, 0.0, 0.0];
        self.queue.write_buffer(
            &self.camera_buf,
            0,
            bytemuck_pod_cast(&cam),
        );
    }

    pub fn ensure_overlay_tex(&mut self, w: u32, h: u32) {
        if self.overlay_tex_size == (w, h) {
            return;
        }
        if let Some(t) = self.overlay_tex.take() {
            t.destroy();
        }
        self.overlay_tex = Some(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text_overlay"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }));
        self.overlay_tex_size = (w, h);
    }

    pub fn upload_overlay_pixels(&self, pixels: &[u8], w: u32, h: u32) {
        let Some(tex) = &self.overlay_tex else { return };
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
}

/// Cast a &[f32] to &[u8] without unsafe — works because f32 is Pod.
fn bytemuck_pod_cast(data: &[f32]) -> &[u8] {
    // SAFETY: f32 has no padding/uninitialized bytes; slice is valid.
    unsafe {
        std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 4)
    }
}

pub fn floats_as_bytes(data: &[f32]) -> &[u8] {
    bytemuck_pod_cast(data)
}
