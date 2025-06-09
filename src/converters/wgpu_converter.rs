use anyhow::{anyhow, Result};
use crate::converters::{YuvToRgbConverter, FrameData};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "wgpu-mode")]
use wgpu::util::DeviceExt;

/// GPU图像处理器
#[cfg(feature = "wgpu-mode")]
pub struct GpuImageProcessor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    gpu_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    cached_size: Option<(u32, u32)>,
    y_buffer: Option<wgpu::Buffer>,
    u_buffer: Option<wgpu::Buffer>,
    v_buffer: Option<wgpu::Buffer>,
    output_buffer: Option<wgpu::Buffer>,
    read_buffer: Option<wgpu::Buffer>,
    capacity: usize,
}

#[cfg(feature = "wgpu-mode")]
impl GpuImageProcessor {
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
            label: Some("GPU YUV to RGB Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/yuv_to_rgb_batch.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GPU Bind Group Layout"),
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
            label: Some("GPU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let gpu_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GPU Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            device,
            queue,
            gpu_pipeline,
            bind_group_layout,
            cached_size: None,
            y_buffer: None,
            u_buffer: None,
            v_buffer: None,
            output_buffer: None,
            read_buffer: None,
            capacity: 0,
        })
    }

    pub async fn convert_yuv420p_to_rgb(
        &mut self,
        frame_data: &[(Vec<u8>, u32, u32)],
    ) -> Result<Vec<Vec<u8>>> {
        if frame_data.is_empty() {
            return Ok(vec![]);
        }

        const GPU_BUFFER_LIMIT: u64 = 134_217_728; // 128MB
        let (width, height) = (frame_data[0].1, frame_data[0].2);
        let frame_output_size = (width * height * 4) as u64;
        let max_single_batch = ((GPU_BUFFER_LIMIT / frame_output_size) as f32 * 0.9) as usize;
        
        let total_frames = frame_data.len();
        if total_frames <= max_single_batch {
            return self.process_batch(frame_data).await;
        }
        
        let mut all_results = Vec::with_capacity(total_frames);
        let sub_batches = (total_frames + max_single_batch - 1) / max_single_batch;
        
        for batch_idx in 0..sub_batches {
            let start_frame = batch_idx * max_single_batch;
            let end_frame = ((batch_idx + 1) * max_single_batch).min(total_frames);
            let sub_batch_data = &frame_data[start_frame..end_frame];
            let batch_results = self.process_batch(sub_batch_data).await?;
            all_results.extend(batch_results);
        }
        
        Ok(all_results)
    }

    async fn process_batch(&mut self, frame_data: &[(Vec<u8>, u32, u32)]) -> Result<Vec<Vec<u8>>> {
        let (first_width, first_height) = (frame_data[0].1, frame_data[0].2);
        if !frame_data.iter().all(|(_, w, h)| *w == first_width && *h == first_height) {
            return Err(anyhow!("所有帧必须具有相同尺寸"));
        }

        let batch_size = frame_data.len();
        let width = first_width;
        let height = first_height;
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        let frame_yuv_size = y_size + 2 * uv_size;

        for (yuv_data, _, _) in frame_data {
            if yuv_data.len() < frame_yuv_size {
                return Err(anyhow!("YUV数据长度不足"));
            }
        }

        let need_new_buffers = self.capacity < batch_size || self.cached_size != Some((width, height));
        if need_new_buffers {
            self.create_buffers(batch_size, width, height).await?;
        }

        let mut batch_y_data = Vec::with_capacity(y_size * batch_size);
        let mut batch_u_data = Vec::with_capacity(uv_size * batch_size);
        let mut batch_v_data = Vec::with_capacity(uv_size * batch_size);
        
        for (yuv_data, _, _) in frame_data {
            batch_y_data.extend_from_slice(&yuv_data[0..y_size]);
            batch_u_data.extend_from_slice(&yuv_data[y_size..y_size + uv_size]);
            batch_v_data.extend_from_slice(&yuv_data[y_size + uv_size..y_size + 2 * uv_size]);
        }

        self.queue.write_buffer(
            self.y_buffer.as_ref().unwrap(),
            0,
            &self.pad_data(&batch_y_data)
        );
        self.queue.write_buffer(
            self.u_buffer.as_ref().unwrap(),
            0,
            &self.pad_data(&batch_u_data)
        );
        self.queue.write_buffer(
            self.v_buffer.as_ref().unwrap(),
            0,
            &self.pad_data(&batch_v_data)
        );

        let params = [width, height, (y_size * batch_size) as u32, (uv_size * batch_size) as u32];
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Parameters Buffer"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.y_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.u_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.v_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.output_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let workgroup_x = (width + 15) / 16;
        let workgroup_y = (height + 15) / 16;
        let workgroup_z = batch_size as u32;
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GPU Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.gpu_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, workgroup_z);
        }

        let total_output_size = (width * height * 4 * batch_size as u32) as u64;
        encoder.copy_buffer_to_buffer(
            self.output_buffer.as_ref().unwrap(),
            0,
            self.read_buffer.as_ref().unwrap(),
            0,
            total_output_size
        );

        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.read_buffer.as_ref().unwrap().slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        receiver.await.map_err(|_| anyhow::anyhow!("无法映射缓冲区"))??;

        let data = buffer_slice.get_mapped_range();
        let frame_rgb_size = (width * height * 3) as usize;
        let mut results = Vec::with_capacity(batch_size);

        for frame_idx in 0..batch_size {
            let mut rgb_data = Vec::with_capacity(frame_rgb_size);
            let frame_offset = frame_idx * (width * height * 4) as usize;
            
            for pixel_idx in 0..(width * height) as usize {
                let rgba_offset = frame_offset + pixel_idx * 4;
                if rgba_offset + 3 < data.len() {
                    let rgba_bytes = &data[rgba_offset..rgba_offset + 4];
                    let rgba_u32 = u32::from_le_bytes([rgba_bytes[0], rgba_bytes[1], rgba_bytes[2], rgba_bytes[3]]);
                    
                    rgb_data.push((rgba_u32 & 0xFF) as u8);
                    rgb_data.push(((rgba_u32 >> 8) & 0xFF) as u8);
                    rgb_data.push(((rgba_u32 >> 16) & 0xFF) as u8);
                }
            }
            results.push(rgb_data);
        }
        
        drop(data);
        self.read_buffer.as_ref().unwrap().unmap();

        Ok(results)
    }

    async fn create_buffers(&mut self, batch_size: usize, width: u32, height: u32) -> Result<()> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;

        let batch_y_size = (y_size * batch_size) as u64;
        let batch_uv_size = (uv_size * batch_size) as u64;
        let batch_rgba_size = (width * height * 4 * batch_size as u32) as u64;

        self.y_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Y Buffer"),
            size: self.pad_size(batch_y_size),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.u_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("U Buffer"),
            size: self.pad_size(batch_uv_size),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.v_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("V Buffer"),
            size: self.pad_size(batch_uv_size),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.output_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: batch_rgba_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));

        self.read_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Read Buffer"),
            size: batch_rgba_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));

        self.capacity = batch_size;
        self.cached_size = Some((width, height));

        Ok(())
    }

    fn pad_size(&self, size: u64) -> u64 {
        (size + 3) & !3
    }

    fn pad_data(&self, data: &[u8]) -> Vec<u8> {
        let mut padded = data.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        padded
    }
}

/// 批处理帧数据
#[derive(Clone)]
pub struct BatchFrameData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 批处理池
pub struct WgpuBatchPool {
    processor: Option<GpuImageProcessor>,
    frame_buffer: Vec<BatchFrameData>,
    batch_size: usize,
    max_wait_time: std::time::Duration,
    last_batch_time: std::time::Instant,
}

impl WgpuBatchPool {
    pub async fn new(batch_size: usize, max_wait_time_ms: u64) -> Result<Self> {
        let processor = match GpuImageProcessor::new().await {
            Ok(p) => Some(p),
            Err(_) => None,
        };
        
        Ok(Self { 
            processor,
            frame_buffer: Vec::with_capacity(batch_size),
            batch_size,
            max_wait_time: std::time::Duration::from_millis(max_wait_time_ms),
            last_batch_time: std::time::Instant::now(),
        })
    }
    
    async fn ensure_processor(&mut self) -> Result<&mut GpuImageProcessor> {
        if self.processor.is_none() {
            self.processor = Some(GpuImageProcessor::new().await?);
        }
        Ok(self.processor.as_mut().unwrap())
    }
    
    pub async fn add_frame(&mut self, frame_data: &FrameData) -> Result<Option<Vec<Vec<u8>>>> {
        let batch_frame = BatchFrameData {
            width: frame_data.width,
            height: frame_data.height,
            data: frame_data.data.clone(),
        };
        
        self.frame_buffer.push(batch_frame);
        
        let should_process = self.frame_buffer.len() >= self.batch_size ||
                           self.last_batch_time.elapsed() >= self.max_wait_time;
        
        if should_process {
            self.process_batch().await
        } else {
            Ok(None)
        }
    }
    
    pub async fn flush(&mut self) -> Result<Option<Vec<Vec<u8>>>> {
        if self.frame_buffer.is_empty() {
            return Ok(None);
        }
        self.process_batch().await
    }
    
    async fn process_batch(&mut self) -> Result<Option<Vec<Vec<u8>>>> {
        if self.frame_buffer.is_empty() {
            return Ok(None);
        }
        
        let frames_to_process: Vec<BatchFrameData> = self.frame_buffer.drain(..).collect();
        self.last_batch_time = std::time::Instant::now();
        
        let processor = self.ensure_processor().await?;
        
        let batch_data: Vec<(Vec<u8>, u32, u32)> = frames_to_process
            .iter()
            .map(|frame| (frame.data.clone(), frame.width, frame.height))
            .collect();
        
        let results = processor.convert_yuv420p_to_rgb(&batch_data).await?;
        
        Ok(Some(results))
    }
}

static GLOBAL_BATCH_POOL: tokio::sync::OnceCell<Arc<Mutex<WgpuBatchPool>>> = tokio::sync::OnceCell::const_new();

pub async fn get_global_batch_pool() -> Arc<Mutex<WgpuBatchPool>> {
    GLOBAL_BATCH_POOL.get_or_init(|| async {
        let pool = WgpuBatchPool::new(8, 100).await
            .expect("Failed to create global WGPU batch pool");
        Arc::new(Mutex::new(pool))
    }).await.clone()
}

/// WGPU转换器（使用批处理优化）
pub struct WgpuBatchConverter {
    use_global_pool: bool,
    local_pool: Option<WgpuBatchPool>,
}

impl WgpuBatchConverter {
    pub async fn new(use_global_pool: bool, batch_size: Option<usize>, max_wait_time_ms: Option<u64>) -> Result<Self> {
        let local_pool = if !use_global_pool {
            Some(WgpuBatchPool::new(
                batch_size.unwrap_or(8),
                max_wait_time_ms.unwrap_or(100)
            ).await?)
        } else {
            None
        };
        
        Ok(Self {
            use_global_pool,
            local_pool,
        })
    }
    
    pub async fn add_frame(&mut self, frame_data: &FrameData) -> Result<Option<Vec<Vec<u8>>>> {
        if self.use_global_pool {
            let pool = get_global_batch_pool().await;
            let mut pool = pool.lock().await;
            pool.add_frame(frame_data).await
        } else if let Some(ref mut pool) = self.local_pool {
            pool.add_frame(frame_data).await
        } else {
            Err(anyhow::anyhow!("No batch pool available"))
        }
    }
    
    pub async fn flush(&mut self) -> Result<Option<Vec<Vec<u8>>>> {
        if self.use_global_pool {
            let pool = get_global_batch_pool().await;
            let mut pool = pool.lock().await;
            pool.flush().await
        } else if let Some(ref mut pool) = self.local_pool {
            pool.flush().await
        } else {
            Ok(None)
        }
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for WgpuBatchConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        // 对于单帧转换，我们添加到批次中并立即刷新
        if let Some(batch_results) = self.add_frame(frame_data).await? {
            // 如果有批次结果，返回最后一个（当前帧）
            if let Some(result) = batch_results.into_iter().last() {
                return Ok(result);
            }
        }
        
        // 如果没有立即得到结果，强制刷新
        if let Some(batch_results) = self.flush().await? {
            if let Some(result) = batch_results.into_iter().last() {
                return Ok(result);
            }
        }
        
        Err(anyhow::anyhow!("批处理转换失败"))
    }

    async fn cleanup(&mut self) -> Result<()> {
        // 清理时刷新所有待处理的帧
        let _ = self.flush().await;
        Ok(())
    }
}


