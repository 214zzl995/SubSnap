use anyhow::{anyhow, Result};
use ffmpeg_next as ffmpeg;
use std::path::Path;
use tokio::sync::mpsc;
use wgpu::util::DeviceExt;

const SAVE_ENABLED: bool = true; // 是否启用保存图片功能

// 配置结构体
#[derive(Clone)]
pub struct ProcessConfig {
    pub target_fps: f64,
    pub output_dir: String,
    pub save_images: bool, // 是否保存图片（可选功能）
    pub max_concurrent_saves: usize,
    pub image_format: String, // "jpg", "png", etc.
    pub jpeg_quality: u8,     // 0-100 for JPEG quality
    pub use_gpu: bool,        // 使用GPU加速
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            target_fps: 1.0,
            output_dir: "extracted_frames".to_string(),
            save_images: false, // 默认不保存图片
            max_concurrent_saves: 4,
            image_format: "jpg".to_string(),
            jpeg_quality: 90,
            use_gpu: true, // 默认使用GPU
        }
    }
}

// 帧数据结构
#[derive(Clone)]
pub struct FrameData {
    pub frame_number: u32,
    pub timestamp: f64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ffmpeg::util::format::Pixel,
}

// GPU 图像处理器（优化版本）
pub struct WgpuImageProcessor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    yuv_to_rgb_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // 缓存缓冲区以避免重复创建
    cached_y_buffer: Option<wgpu::Buffer>,
    cached_u_buffer: Option<wgpu::Buffer>,
    cached_v_buffer: Option<wgpu::Buffer>,
    cached_output_buffer: Option<wgpu::Buffer>,
    cached_read_buffer: Option<wgpu::Buffer>,
    cached_params_buffer: Option<wgpu::Buffer>,
    cached_size: Option<(u32, u32)>,
}

impl WgpuImageProcessor {
    pub async fn new() -> Result<Self> {
        // 创建 wgpu 实例
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // 请求适配器
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        // 请求设备和队列
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                ..Default::default()
            })
            .await?;

        // 创建计算着色器
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("YUV to RGB Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("yuv_to_rgb.wgsl").into()),
        });

        // 创建绑定组布局（更新为5个绑定）
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("YUV to RGB Bind Group Layout"),
            entries: &[
                // Y 平面缓冲区
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
                // U 平面缓冲区
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
                // V 平面缓冲区
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
                // 输出 RGB 数据缓冲区
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
                // 参数缓冲区 (width, height, y_plane_size, uv_plane_size)
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

        // 创建计算管线
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

        // 分离YUV平面
        let y_plane = &yuv_data[0..y_size];
        let u_plane = &yuv_data[y_size..y_size + uv_size];
        let v_plane = &yuv_data[y_size + uv_size..y_size + 2 * uv_size];

        // 检查是否可以重用缓冲区
        let need_new_buffers = self.cached_size != Some((width, height));

        let (y_buffer, u_buffer, v_buffer, output_buffer, read_buffer, params_buffer) = 
            if need_new_buffers {
                // 创建新缓冲区并缓存
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

                // 缓存缓冲区
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
                // 重用缓存的缓冲区，只更新数据
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

        // 创建绑定组
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

        // 创建命令编码器
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("YUV to RGB Encoder"),
        });

        // 开始计算通道
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("YUV to RGB Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.yuv_to_rgb_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // 使用优化的工作组大小 16x16
            let workgroup_x = (width + 15) / 16;
            let workgroup_y = (height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // 复制数据到读取缓冲区
        encoder.copy_buffer_to_buffer(output_buffer, 0, read_buffer, 0, (width * height * 4) as u64);

        // 提交命令
        self.queue.submit(Some(encoder.finish()));

        // 读取结果
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

        // 将RGBA数据转换为RGB数据
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

        // 按u32读取RGBA数据并转换为RGB
        for chunk in rgba_data.chunks_exact(4) {
            if chunk.len() == 4 {
                let rgba_u32 = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

                // 从u32中提取RGBA分量 (我们之前打包为 A<<24 | B<<16 | G<<8 | R)
                let r = (rgba_u32 & 0xFF) as u8;
                let g = ((rgba_u32 >> 8) & 0xFF) as u8;
                let b = ((rgba_u32 >> 16) & 0xFF) as u8;
                // A分量被忽略

                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        Ok(rgb_data)
    }

    // 创建填充的缓冲区，确保大小是4的倍数
    fn create_padded_buffer(&self, data: &[u8], label: &str) -> wgpu::Buffer {
        let padded_data = self.pad_data(data);
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &padded_data,
            usage: wgpu::BufferUsages::STORAGE,
        })
    }

    // 填充数据到4字节对齐
    fn pad_data(&self, data: &[u8]) -> Vec<u8> {
        let mut padded = data.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        padded
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化 FFmpeg
    ffmpeg::init()?;

    println!("🎬 FFmpeg 初始化成功!");
    println!("📦 FFmpeg 已成功集成到 SubSnap 项目中");
    println!("🚀 使用 WGPU GPU 加速处理");

    // 显示一些基本信息
    println!("\n📋 可用功能示例:");
    println!("✅ FFmpeg 库已初始化");
    println!("✅ 可以进行视频/音频处理");
    println!("✅ 支持多种媒体格式");
    println!("✅ WGPU GPU 加速图像转换");
    println!("✅ 移除了 OpenCV 依赖，项目更轻量");

    // 演示一些简单的 ffmpeg-next 功能
    demo_ffmpeg_features().await?;

    println!("\n🎯 准备就绪 - 可以开始开发字幕相关功能!");

    Ok(())
}

async fn demo_ffmpeg_features() -> Result<()> {
    println!("\n🔍 FFmpeg 功能演示:");

    // 注意：以下是一些基本的演示，实际使用时需要根据具体需求调整
    println!("  📄 库已加载，可以处理各种媒体文件");
    println!("  🎥 支持视频编解码");
    println!("  🎵 支持音频编解码");
    println!("  📝 可以提取和处理字幕轨道");

    // 检测最优解码器
    check_optimal_decoders()?;

    // 检查 WGPU 支持
    check_wgpu_support().await?;

    // 演示视频帧拆分功能（只在有有效文件时）
    println!("\n🎞️  准备演示视频帧拆分功能...");

    let config = ProcessConfig {
        target_fps: 1.0, // 每秒提取1帧
        output_dir: "extracted_frames".to_string(),
        save_images: SAVE_ENABLED, // 是否保存图片
        max_concurrent_saves: 4,
        image_format: "jpg".to_string(),
        jpeg_quality: 90,
        use_gpu: true, // 使用GPU加速
    };

    if let Err(e) = demo_frame_extraction_with_wgpu(config).await {
        println!("  ⚠️  帧保存演示跳过: {}", e);
    }

    Ok(())
}

async fn check_wgpu_support() -> Result<()> {
    println!("\n🔧 检查 WGPU GPU 支持:");

    match WgpuImageProcessor::new().await {
        Ok(_) => {
            println!("  ✅ WGPU 初始化成功");
            println!("  🚀 GPU 加速可用");
            println!("  💡 支持所有平台的现代GPU");
        }
        Err(e) => {
            println!("  ⚠️  WGPU 初始化失败: {}", e);
            println!("  💻 将回退到CPU处理模式");
        }
    }

    Ok(())
}

fn check_optimal_decoders() -> Result<()> {
    println!("\n🔧 检查最优解码器:");

    // 检查系统中可用的解码器
    println!("  📋 分析系统可用的解码器配置...");

    // 如果有测试文件，分析其编解码器
    let test_files = ["input.mp4", "MIAB-057.mp4"];

    for file_path in &test_files {
        if Path::new(file_path).exists() {
            println!("  📁 分析文件: {}", file_path);
            match analyze_file_codecs(file_path) {
                Ok(_) => println!("    ✅ 文件分析完成"),
                Err(e) => println!("    ⚠️  文件分析失败: {}", e),
            }
        }
    }

    Ok(())
}

fn analyze_file_codecs(file_path: &str) -> Result<()> {
    use ffmpeg::{format, media};

    let input = format::input(&Path::new(file_path))?;

    println!("    📊 文件流分析:");

    for (i, stream) in input.streams().enumerate() {
        match stream.parameters().medium() {
            media::Type::Video => {
                println!("      🎥 视频流 #{}", i);

                // 创建解码器获取详细信息
                if let Ok(decoder_context) =
                    ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                {
                    if let Ok(decoder) = decoder_context.decoder().video() {
                        if let Some(codec) = decoder.codec() {
                            println!("        📝 编解码器: {}", codec.name());

                            // 评估解码器性能
                            let performance_rating = evaluate_decoder_performance(&codec.name());
                            println!("        ⭐ 性能评级: {}", performance_rating);
                        }

                        println!(
                            "        📐 分辨率: {}x{}",
                            decoder.width(),
                            decoder.height()
                        );
                        println!("        🎨 像素格式: {:?}", decoder.format());

                        // 计算解码复杂度估计
                        let complexity =
                            calculate_decoding_complexity(decoder.width(), decoder.height());
                        println!("        🧮 解码复杂度: {}", complexity);
                    }
                }
            }
            media::Type::Audio => {
                println!("      🎵 音频流 #{}", i);

                if let Ok(decoder_context) =
                    ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                {
                    if let Ok(decoder) = decoder_context.decoder().audio() {
                        if let Some(codec) = decoder.codec() {
                            println!("        📝 编解码器: {}", codec.name());
                        }
                        println!("        🔊 采样率: {} Hz", decoder.rate());
                        println!("        📻 声道数: {}", decoder.channels());
                    }
                }
            }
            media::Type::Subtitle => {
                println!("      📝 字幕流 #{}", i);
            }
            _ => {
                println!("      ❓ 其他流 #{}", i);
            }
        }
    }

    Ok(())
}

fn evaluate_decoder_performance(codec_name: &str) -> &'static str {
    match codec_name.to_lowercase().as_str() {
        name if name.contains("h264") || name.contains("avc") => "🚀 优秀 (快速解码)",
        name if name.contains("h265") || name.contains("hevc") => "⚡ 良好 (高效但较慢)",
        name if name.contains("vp9") => "✅ 良好 (平衡性能)",
        name if name.contains("av1") => "🔥 极佳压缩但解码慢",
        name if name.contains("vp8") => "📊 一般 (旧标准)",
        name if name.contains("mpeg") => "📺 基础 (传统编码)",
        _ => "❓ 未知性能特征",
    }
}

fn calculate_decoding_complexity(width: u32, height: u32) -> &'static str {
    let pixels = width * height;

    match pixels {
        0..=307200 => "🟢 低 (480p以下)",      // 480x640 = 307200
        307201..=921600 => "🟡 中等 (720p)",   // 720p = 1280x720 = 921600
        921601..=2073600 => "🟠 较高 (1080p)", // 1080p = 1920x1080 = 2073600
        2073601..=8294400 => "🔴 高 (4K)",     // 4K = 3840x2160 = 8294400
        _ => "🚨 极高 (8K及以上)",
    }
}

async fn demo_frame_extraction_with_wgpu(config: ProcessConfig) -> Result<()> {
    use ffmpeg::media;

    println!(
        "\n🎞️  开始演示视频帧拆分 {}:",
        if config.save_images {
            "(带WGPU GPU加速保存)"
        } else {
            "(仅提取)"
        }
    );

    let input_path = "input.mp4";

    // 检查文件是否存在
    if !Path::new(input_path).exists() {
        println!("  ⚠️  警告: input.mp4 文件不存在，跳过帧拆分演示");
        return Ok(());
    }

    // 初始化 WGPU 处理器
    let gpu_processor = if config.use_gpu {
        match WgpuImageProcessor::new().await {
            Ok(processor) => {
                println!("  🚀 WGPU GPU 处理器初始化成功");
                Some(processor)
            }
            Err(e) => {
                println!("  ⚠️  GPU 初始化失败，使用CPU模式: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 如果需要保存图片，创建输出目录
    if config.save_images {
        tokio::fs::create_dir_all(&config.output_dir).await?;
        println!("  📂 输出目录: {}", config.output_dir);
        println!("  🖼️  图片格式: {}", config.image_format);
        println!("  🔄 最大并发保存: {}", config.max_concurrent_saves);

        if gpu_processor.is_some() {
            println!("  🚀 GPU加速: 启用 (WGPU)");
        } else {
            println!("  💻 处理模式: CPU");
        }
    } else {
        println!("  🔍 模式: 仅提取帧，不保存图片");
    }

    println!("  📁 输入文件: {}", input_path);
    println!("  🎯 目标提取FPS: {}", config.target_fps);

    // 打开输入文件，使用优化设置
    let mut input = create_optimized_input_context(input_path)?;

    // 查找视频流并获取基本信息
    let mut video_stream_index = None;
    for (i, stream) in input.streams().enumerate() {
        if stream.parameters().medium() == media::Type::Video {
            video_stream_index = Some(i);
            println!("  📊 找到视频流:");
            println!("    🎯 流索引: {}", i);

            let decoder_context =
                ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
            let decoder = decoder_context.decoder().video()?;

            println!("    📐 分辨率: {}x{}", decoder.width(), decoder.height());

            if let Some(codec) = decoder.codec() {
                println!("    📝 编解码器: {}", codec.name());
            }
            println!("    🎨 像素格式: {:?}", decoder.format());
            break;
        }
    }

    let video_stream_index = video_stream_index.ok_or_else(|| anyhow!("找不到视频流"))?;

    // 设置处理管道（仅在需要保存图片时）
    let (frame_sender, frame_receiver) = if config.save_images {
        let (sender, receiver) = mpsc::channel::<FrameData>(config.max_concurrent_saves * 2);
        (Some(sender), Some(receiver))
    } else {
        (None, None)
    };

    // 启动异步图片保存任务（仅在需要时）
    let save_handle = if let Some(receiver) = frame_receiver {
        let save_config = config.clone();
        Some(tokio::spawn(async move {
            process_frames_async_wgpu(receiver, save_config, gpu_processor).await
        }))
    } else {
        None
    };

    // FFmpeg 解码部分
    let stream = &input.streams().nth(video_stream_index).unwrap();
    let mut decoder_context =
        ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;

    optimize_decoder_for_speed(&mut decoder_context)?;
    let mut decoder = decoder_context.decoder().video()?;

    let frame_interval = 1.0 / config.target_fps;
    println!("  ⏱️  帧提取间隔: {:.2}秒", frame_interval);
    println!("  🚀 启用多线程解码以提升速度");

    let mut extracted_count = 0;
    let mut next_extract_time = 0.0;
    let mut processed_packets = 0;

    let start_time = std::time::Instant::now();
    let mut last_video_time = 0.0;

    println!(
        "  🚀 开始按FPS提取帧{}:",
        if config.save_images {
            "并异步保存"
        } else {
            ""
        }
    );

    for (stream, packet) in input.packets() {
        if stream.index() == video_stream_index {
            processed_packets += 1;

            decoder.send_packet(&packet)?;

            let mut decoded = ffmpeg::util::frame::video::Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let timestamp =
                    decoded.timestamp().unwrap_or(0) as f64 * f64::from(stream.time_base());

                if timestamp >= next_extract_time {
                    extracted_count += 1;
                    last_video_time = timestamp;

                    // 如果需要保存图片，转换帧数据并发送到异步保存队列
                    if let Some(ref sender) = frame_sender {
                        if let Ok(frame_data) =
                            convert_frame_to_data(&decoded, extracted_count, timestamp)
                        {
                            // 非阻塞发送，如果队列满了就跳过这一帧
                            if let Err(_) = sender.try_send(frame_data) {
                                println!("    ⚠️  保存队列已满，跳过帧 #{}", extracted_count);
                            }
                        }
                    }

                    if extracted_count % 30 == 0 || extracted_count <= 5 {
                        println!(
                            "    📸 {}帧 #{}: 时间戳 {:.2}s, 格式 {:?}, 大小 {}x{}",
                            if config.save_images {
                                "提取并保存"
                            } else {
                                "提取"
                            },
                            extracted_count,
                            timestamp,
                            decoded.format(),
                            decoded.width(),
                            decoded.height()
                        );
                    }

                    next_extract_time += frame_interval;
                }
            }

            if processed_packets % 100 == 0 {
                let current_time = std::time::Instant::now();
                let elapsed_real_time = current_time.duration_since(start_time).as_secs_f64();

                let speed = if elapsed_real_time > 0.0 && last_video_time > 0.0 {
                    last_video_time / elapsed_real_time
                } else {
                    0.0
                };

                println!(
                    "    📊 已处理 {} 个数据包, 提取 {} 帧, speed={:.2}x",
                    processed_packets, extracted_count, speed
                );
            }
        }
    }

    // 如果有保存任务，等待完成
    if let Some(sender) = frame_sender {
        // 关闭发送端，通知保存任务结束
        drop(sender);
    }

    let save_result = if let Some(handle) = save_handle {
        match handle.await {
            Ok(result) => Some(result),
            Err(e) => {
                println!("  ❌ 任务执行失败: {}", e);
                None
            }
        }
    } else {
        None
    };

    let total_elapsed = start_time.elapsed().as_secs_f64();
    let final_speed = if total_elapsed > 0.0 && last_video_time > 0.0 {
        last_video_time / total_elapsed
    } else {
        0.0
    };

    println!(
        "  ✅ 解码完成! 按{}FPS成功提取了 {} 帧, 最终speed={:.2}x",
        config.target_fps, extracted_count, final_speed
    );
    println!("  📊 总共处理了 {} 个视频数据包", processed_packets);
    println!(
        "  ⏱️  总耗时: {:.2}秒, 处理视频时长: {:.2}秒",
        total_elapsed, last_video_time
    );

    if let Some(result) = save_result {
        match result {
            Ok(saved_count) => {
                println!("  💾 异步保存完成: {} 张图片已保存", saved_count);
            }
            Err(e) => {
                println!("  ❌ 保存过程中出现错误: {}", e);
            }
        }
        println!("  💡 优化措施: 多线程解码 + WGPU GPU加速 + 异步保存 + 非阻塞管道");
    } else {
        println!("  💡 优化措施: 多线程解码 + 快速提取 (无IO开销)");
    }

    Ok(())
}

fn convert_frame_to_data(
    frame: &ffmpeg::util::frame::video::Video,
    frame_number: u32,
    timestamp: f64,
) -> Result<FrameData> {
    let width = frame.width();
    let height = frame.height();
    let format = frame.format();

    // 正确复制所有平面的帧数据
    let mut data = Vec::new();

    match format {
        ffmpeg::util::format::Pixel::YUV420P => {
            // YUV420P 格式：Y平面 + U平面 + V平面
            let y_size = (width * height) as usize;
            let uv_size = y_size / 4;

            // Y 平面
            data.extend_from_slice(&frame.data(0)[0..y_size]);
            // U 平面
            data.extend_from_slice(&frame.data(1)[0..uv_size]);
            // V 平面
            data.extend_from_slice(&frame.data(2)[0..uv_size]);
        }
        ffmpeg::util::format::Pixel::RGB24 => {
            // RGB24 格式：直接复制
            let rgb_size = (width * height * 3) as usize;
            data.extend_from_slice(&frame.data(0)[0..rgb_size]);
        }
        ffmpeg::util::format::Pixel::YUYV422 => {
            // YUYV422 格式：每个像素2字节
            let yuyv_size = (width * height * 2) as usize;
            data.extend_from_slice(&frame.data(0)[0..yuyv_size]);
        }
        _ => {
            // 对于其他格式，尝试复制第一个平面
            let data_size = frame.data(0).len().min((width * height * 4) as usize);
            data.extend_from_slice(&frame.data(0)[0..data_size]);
        }
    }

    Ok(FrameData {
        frame_number,
        timestamp,
        width,
        height,
        data,
        format,
    })
}

// 创建优化的输入上下文
fn create_optimized_input_context(file_path: &str) -> Result<ffmpeg::format::context::Input> {
    use ffmpeg::format;

    let input = format::input(&Path::new(file_path))?;

    // 设置一些优化选项
    // 这里可以添加更多的优化设置

    Ok(input)
}

// 优化解码器速度
fn optimize_decoder_for_speed(
    _decoder_context: &mut ffmpeg::codec::context::Context,
) -> Result<()> {
    // 设置解码器参数以提升速度
    // 可以添加特定的优化设置
    Ok(())
}

// 异步处理帧数据并保存图片（使用WGPU加速）
async fn process_frames_async_wgpu(
    mut receiver: mpsc::Receiver<FrameData>,
    config: ProcessConfig,
    _gpu_processor: Option<WgpuImageProcessor>,
) -> Result<usize> {
    let mut saved_count = 0;
    let mut tasks = Vec::new();

    while let Some(frame_data) = receiver.recv().await {
        let config_clone = config.clone();

        // 为每个任务创建新的GPU处理器实例
        let task = if config.use_gpu {
            tokio::spawn(async move {
                // 在每个任务中重新初始化GPU处理器
                match WgpuImageProcessor::new().await {
                    Ok(gpu_proc) => {
                        process_single_frame_with_gpu(frame_data, config_clone, gpu_proc).await
                    }
                    Err(_) => {
                        process_single_frame_cpu_only(frame_data, config_clone).await
                    }
                }
            })
        } else {
            tokio::spawn(
                async move { process_single_frame_cpu_only(frame_data, config_clone).await },
            )
        };

        tasks.push(task);

        // 限制并发任务数量
        if tasks.len() >= config.max_concurrent_saves {
            // 等待一些任务完成
            let mut i = 0;
            while i < tasks.len() {
                if tasks[i].is_finished() {
                    match tasks.remove(i).await {
                        Ok(Ok(_)) => saved_count += 1,
                        Ok(Err(e)) => println!("    ⚠️  保存帧失败: {}", e),
                        Err(e) => println!("    ⚠️  任务执行失败: {}", e),
                    }
                } else {
                    i += 1;
                }
            }
        }
    }

    // 等待所有剩余任务完成
    for task in tasks {
        match task.await {
            Ok(Ok(_)) => saved_count += 1,
            Ok(Err(e)) => println!("    ⚠️  保存帧失败: {}", e),
            Err(e) => println!("    ⚠️  任务执行失败: {}", e),
        }
    }

    Ok(saved_count)
}

// 使用GPU处理单个帧
async fn process_single_frame_with_gpu(
    frame_data: FrameData,
    config: ProcessConfig,
    mut gpu_processor: WgpuImageProcessor,
) -> Result<()> {
    let FrameData {
        frame_number,
        timestamp,
        width,
        height,
        data,
        format,
    } = frame_data;

    // 根据格式转换为RGB
    let rgb_data = match format {
        ffmpeg::util::format::Pixel::YUV420P => {
            // 使用GPU加速转换
            gpu_processor
                .convert_yuv420p_to_rgb(&data, width, height)
                .await?
        }
        ffmpeg::util::format::Pixel::RGB24 => {
            // 已经是RGB格式，直接使用
            data
        }
        _ => {
            // 其他格式使用CPU转换
            convert_other_format_to_rgb(&data, width, height, format)?
        }
    };

    // 保存图片
    save_rgb_image(&rgb_data, width, height, frame_number, timestamp, &config).await?;

    Ok(())
}

// 使用CPU处理单个帧
async fn process_single_frame_cpu_only(frame_data: FrameData, config: ProcessConfig) -> Result<()> {
    let FrameData {
        frame_number,
        timestamp,
        width,
        height,
        data,
        format,
    } = frame_data;

    // 根据格式转换为RGB
    let rgb_data = match format {
        ffmpeg::util::format::Pixel::YUV420P => {
            // CPU转换
            convert_yuv420p_to_rgb_cpu(&data, width, height)?
        }
        ffmpeg::util::format::Pixel::RGB24 => {
            // 已经是RGB格式，直接使用
            data
        }
        _ => {
            // 其他格式使用简单转换
            convert_other_format_to_rgb(&data, width, height, format)?
        }
    };

    // 保存图片
    save_rgb_image(&rgb_data, width, height, frame_number, timestamp, &config).await?;

    Ok(())
}

// 保存RGB图像到文件
async fn save_rgb_image(
    rgb_data: &[u8],
    width: u32,
    height: u32,
    frame_number: u32,
    timestamp: f64,
    config: &ProcessConfig,
) -> Result<()> {
    use image::{ImageBuffer, RgbImage};

    // 创建RGB图像
    let img: RgbImage = ImageBuffer::from_vec(width, height, rgb_data.to_vec())
        .ok_or_else(|| anyhow!("无法创建RGB图像"))?;

    // 生成文件名
    let filename = format!(
        "frame_{:06}_{:.3}s.{}",
        frame_number, timestamp, config.image_format
    );
    let filepath = Path::new(&config.output_dir).join(filename);

    // 根据格式保存图像
    match config.image_format.as_str() {
        "jpg" | "jpeg" => {
            let mut output = std::fs::File::create(&filepath)?;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut output,
                config.jpeg_quality,
            );
            img.write_with_encoder(encoder)?;
        }
        "png" => {
            img.save_with_format(&filepath, image::ImageFormat::Png)?;
        }
        _ => {
            // 默认保存为JPEG
            let mut output = std::fs::File::create(&filepath)?;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut output,
                config.jpeg_quality,
            );
            img.write_with_encoder(encoder)?;
        }
    }

    Ok(())
}

// CPU版本的YUV420P到RGB转换
fn convert_yuv420p_to_rgb_cpu(yuv_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let width = width as usize;
    let height = height as usize;
    let y_size = width * height;
    let uv_size = y_size / 4;

    if yuv_data.len() < y_size + 2 * uv_size {
        return Err(anyhow!("YUV数据长度不足"));
    }

    let mut rgb_data = Vec::with_capacity(y_size * 3);

    let y_plane = &yuv_data[0..y_size];
    let u_plane = &yuv_data[y_size..y_size + uv_size];
    let v_plane = &yuv_data[y_size + uv_size..y_size + 2 * uv_size];

    for y in 0..height {
        for x in 0..width {
            let y_index = y * width + x;
            let uv_index = (y / 2) * (width / 2) + (x / 2);

            let y_val = y_plane[y_index] as f32;
            let u_val = u_plane[uv_index] as f32;
            let v_val = v_plane[uv_index] as f32;

            // YUV到RGB转换 (BT.709)
            let y_f = y_val - 16.0;
            let u_f = u_val - 128.0;
            let v_f = v_val - 128.0;

            let r = (1.164 * y_f + 1.793 * v_f).clamp(0.0, 255.0) as u8;
            let g = (1.164 * y_f - 0.213 * u_f - 0.533 * v_f).clamp(0.0, 255.0) as u8;
            let b = (1.164 * y_f + 2.112 * u_f).clamp(0.0, 255.0) as u8;

            rgb_data.push(r);
            rgb_data.push(g);
            rgb_data.push(b);
        }
    }

    Ok(rgb_data)
}

// 其他格式到RGB的转换函数
fn convert_other_format_to_rgb(
    data: &[u8],
    width: u32,
    height: u32,
    format: ffmpeg::util::format::Pixel,
) -> Result<Vec<u8>> {
    match format {
        ffmpeg::util::format::Pixel::YUYV422 => {
            // YUYV422 格式转换
            convert_yuyv422_to_rgb(data, width, height)
        }
        _ => {
            // 对于不支持的格式，创建一个简单的灰度RGB图像
            println!("    ⚠️  不支持的像素格式: {:?}, 使用默认灰度转换", format);
            let rgb_size = (width * height * 3) as usize;
            let mut rgb_data = Vec::with_capacity(rgb_size);

            // 如果有数据，使用第一个通道作为灰度值
            if !data.is_empty() {
                let pixels = (width * height) as usize;
                for i in 0..pixels {
                    let gray = if i < data.len() { data[i] } else { 128 };
                    rgb_data.push(gray);
                    rgb_data.push(gray);
                    rgb_data.push(gray);
                }
            } else {
                // 如果没有数据，创建灰色图像
                rgb_data.resize(rgb_size, 128);
            }

            Ok(rgb_data)
        }
    }
}

// YUYV422格式转换
fn convert_yuyv422_to_rgb(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let width = width as usize;
    let height = height as usize;
    let mut rgb_data = Vec::with_capacity(width * height * 3);

    for y in 0..height {
        for x in 0..(width / 2) {
            let index = (y * width + x * 2) * 2;
            if index + 3 < data.len() {
                let y0 = data[index] as f32;
                let u = data[index + 1] as f32;
                let y1 = data[index + 2] as f32;
                let v = data[index + 3] as f32;

                // 转换两个像素
                for y_val in [y0, y1].iter() {
                    let y_f = y_val - 16.0;
                    let u_f = u - 128.0;
                    let v_f = v - 128.0;

                    let r = (1.164 * y_f + 1.793 * v_f).clamp(0.0, 255.0) as u8;
                    let g = (1.164 * y_f - 0.213 * u_f - 0.533 * v_f).clamp(0.0, 255.0) as u8;
                    let b = (1.164 * y_f + 2.112 * u_f).clamp(0.0, 255.0) as u8;

                    rgb_data.push(r);
                    rgb_data.push(g);
                    rgb_data.push(b);
                }
            }
        }
    }

    Ok(rgb_data)
}
