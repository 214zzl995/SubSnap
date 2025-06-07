#[cfg(feature = "wgpu-mode")]
use anyhow::{anyhow, Result};
#[cfg(feature = "wgpu-mode")]
use wgpu::util::DeviceExt;

#[cfg(feature = "wgpu-mode")]
pub struct WgpuImageProcessor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    yuv_to_rgb_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    cached_y_buffer: Option<wgpu::Buffer>,
    cached_u_buffer: Option<wgpu::Buffer>,
    cached_v_buffer: Option<wgpu::Buffer>,
    cached_output_buffer: Option<wgpu::Buffer>,
    cached_read_buffer: Option<wgpu::Buffer>,
    cached_params_buffer: Option<wgpu::Buffer>,
    cached_size: Option<(u32, u32)>,
}

#[cfg(feature = "wgpu-mode")]
impl WgpuImageProcessor {
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                ..Default::default()
            })
            .await?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("YUV to RGB Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("yuv_to_rgb.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("YUV to RGB Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("YUV to RGB Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let yuv_to_rgb_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("YUV to RGB Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            device,
            queue,
            yuv_to_rgb_pipeline,
            bind_group_layout,
            cached_y_buffer: None,
            cached_u_buffer: None,
            cached_v_buffer: None,
            cached_output_buffer: None,
            cached_read_buffer: None,
            cached_params_buffer: None,
            cached_size: None,
        })
    }

    pub async fn convert_yuv420p_to_rgb(
        &mut self,
        yuv_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        
        if yuv_data.len() < y_size + 2 * uv_size {
            return Err(anyhow!("YUV数据长度不足"));
        }

        let y_plane = &yuv_data[0..y_size];
        let u_plane = &yuv_data[y_size..y_size + uv_size];
        let v_plane = &yuv_data[y_size + uv_size..y_size + 2 * uv_size];

        let need_new_buffers = self.cached_size != Some((width, height));

        let (y_buffer, u_buffer, v_buffer, output_buffer, read_buffer, params_buffer) = 
            if need_new_buffers {
                let y_buf = self.create_padded_buffer(y_plane, "Y Plane Buffer");
                let u_buf = self.create_padded_buffer(u_plane, "U Plane Buffer");
                let v_buf = self.create_padded_buffer(v_plane, "V Plane Buffer");
                
                let rgba_size = (width * height * 4) as u64;
                let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("RGBA Output Buffer"),
                    size: rgba_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                let read_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Read Buffer"),
                    size: rgba_size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });

                let params = [width, height, y_size as u32, uv_size as u32];
                let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Parameters Buffer"),
                    contents: bytemuck::cast_slice(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                self.cached_y_buffer = Some(y_buf);
                self.cached_u_buffer = Some(u_buf);
                self.cached_v_buffer = Some(v_buf);
                self.cached_output_buffer = Some(output_buf);
                self.cached_read_buffer = Some(read_buf);
                self.cached_params_buffer = Some(params_buf);
                self.cached_size = Some((width, height));

                (
                    self.cached_y_buffer.as_ref().unwrap(),
                    self.cached_u_buffer.as_ref().unwrap(),
                    self.cached_v_buffer.as_ref().unwrap(),
                    self.cached_output_buffer.as_ref().unwrap(),
                    self.cached_read_buffer.as_ref().unwrap(),
                    self.cached_params_buffer.as_ref().unwrap(),
                )
            } else {
                self.queue.write_buffer(self.cached_y_buffer.as_ref().unwrap(), 0, 
                    &self.pad_data(y_plane));
                self.queue.write_buffer(self.cached_u_buffer.as_ref().unwrap(), 0, 
                    &self.pad_data(u_plane));
                self.queue.write_buffer(self.cached_v_buffer.as_ref().unwrap(), 0, 
                    &self.pad_data(v_plane));

                (
                    self.cached_y_buffer.as_ref().unwrap(),
                    self.cached_u_buffer.as_ref().unwrap(),
                    self.cached_v_buffer.as_ref().unwrap(),
                    self.cached_output_buffer.as_ref().unwrap(),
                    self.cached_read_buffer.as_ref().unwrap(),
                    self.cached_params_buffer.as_ref().unwrap(),
                )
            };

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("YUV to RGB Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: u_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: v_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("YUV to RGB Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("YUV to RGB Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.yuv_to_rgb_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroup_x = (width + 15) / 16;
            let workgroup_y = (height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        encoder.copy_buffer_to_buffer(output_buffer, 0, read_buffer, 0, (width * height * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = read_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        receiver.await.map_err(|_| anyhow::anyhow!("Failed to receive buffer mapping result"))??;

        let data = buffer_slice.get_mapped_range();
        let rgba_data = data.to_vec();
        drop(data);
        read_buffer.unmap();

        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

        for chunk in rgba_data.chunks_exact(4) {
            if chunk.len() == 4 {
                let rgba_u32 = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let r = (rgba_u32 & 0xFF) as u8;
                let g = ((rgba_u32 >> 8) & 0xFF) as u8;
                let b = ((rgba_u32 >> 16) & 0xFF) as u8;

                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        Ok(rgb_data)
    }

    fn create_padded_buffer(&self, data: &[u8], label: &str) -> wgpu::Buffer {
        let padded_data = self.pad_data(data);
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &padded_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn pad_data(&self, data: &[u8]) -> Vec<u8> {
        let mut padded = data.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        padded
    }
} 