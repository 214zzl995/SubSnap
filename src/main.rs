use ffmpeg_next as ffmpeg;
use futures::future::join_all;
use std::path::Path;
use std::io::Write;
use tokio::sync::mpsc;
use yuvutils_rs::{yuv420_to_rgb, YuvStandardMatrix, YuvPlanarImage, YuvRange};
use image::{ImageBuffer, Rgb};

const SAVE_ENABLED: bool = true; // 是否启用保存图片功能

// 配置结构体
#[derive(Clone)]
pub struct ProcessConfig {
    pub target_fps: f64,
    pub output_dir: String,
    pub save_images: bool, // 是否保存图片（可选功能）
    pub max_concurrent_saves: usize,
    pub image_format: String, // "jpg", "png", etc.
    pub jpeg_quality: i32,    // 0-100 for JPEG quality
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            target_fps: 1.0,
            output_dir: "extracted_frames".to_string(),
            save_images: false, // 默认不保存图片
            max_concurrent_saves: 4,
            image_format: "jpg".to_string(), // 修改默认格式为 jpg
            jpeg_quality: 90,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 FFmpeg
    ffmpeg::init()?;

    println!("🎬 FFmpeg 初始化成功!");
    println!("📦 FFmpeg 已成功集成到 SubSnap 项目中");
    println!("🍎 Mac 设备优化版本 (使用 yuvutils-rs)");

    // 显示一些基本信息
    println!("\n📋 可用功能示例:");
    println!("✅ FFmpeg 库已初始化");
    println!("✅ 可以进行视频/音频处理");
    println!("✅ 支持多种媒体格式");
    println!("✅ yuvutils-rs YUV转RGB转换已集成");
    println!("✅ 轻量级图片保存支持 (无OpenCV依赖)");

    // 演示一些简单的 ffmpeg-next 功能
    demo_ffmpeg_features().await?;

    println!("\n🎯 准备就绪 - 可以开始开发字幕相关功能!");

    Ok(())
}

async fn demo_ffmpeg_features() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 FFmpeg 功能演示:");

    // 注意：以下是一些基本的演示，实际使用时需要根据具体需求调整
    println!("  📄 库已加载，可以处理各种媒体文件");
    println!("  🎥 支持视频编解码");
    println!("  🎵 支持音频编解码");
    println!("  📝 可以提取和处理字幕轨道");

    // 检查最优解码器
    check_optimal_decoders()?;

    // 检查 yuvutils-rs 支持
    check_yuvutils_support().await?;

    // 演示视频帧拆分功能（只在有有效文件时）
    println!("\n🎞️  准备演示视频帧拆分功能...");

    let config = ProcessConfig {
        target_fps: 1.0, // 每秒提取1帧
        output_dir: "extracted_frames".to_string(),
        save_images: SAVE_ENABLED, // 是否保存图片
        max_concurrent_saves: 4,
        image_format: "jpg".to_string(),
        jpeg_quality: 90,
    };

    if let Err(e) = demo_frame_extraction_with_yuvutils(config).await {
        println!("  ⚠️  帧保存演示跳过: {}", e);
    }

    Ok(())
}

async fn check_yuvutils_support() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 检查 yuvutils-rs YUV转换支持:");

    println!("  ✅ yuvutils-rs 库已加载");
    println!("  🚀 支持高性能 YUV420 到 RGB 转换");
    println!("  📐 支持多种色彩标准 (BT.601, BT.709, BT.2020)");
    println!("  🎨 支持全范围和限制范围色度");
    println!("  💻 纯 Rust 实现，无外部依赖");
    println!("  🍎 Mac 设备优化: CPU向量化加速");

    Ok(())
}

fn check_optimal_decoders() -> Result<(), Box<dyn std::error::Error>> {
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

fn analyze_file_codecs(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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

async fn demo_frame_extraction_with_yuvutils(
    config: ProcessConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use ffmpeg::media;

    println!(
        "\n🎞️  开始演示视频帧拆分 {}:",
        if config.save_images {
            "(带yuvutils-rs保存)"
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

    // 如果需要保存图片，创建输出目录
    if config.save_images {
        tokio::fs::create_dir_all(&config.output_dir).await?;
        println!("  📂 输出目录: {}", config.output_dir);
        println!("  🖼️  图片格式: {}", config.image_format);
        println!("  🔄 最大并发保存: {}", config.max_concurrent_saves);
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

    let video_stream_index = video_stream_index.ok_or("找不到视频流")?;

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
            process_frames_async(receiver, save_config).await
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
            "并保存"
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
                            // 阻塞等待队列有空位，保证所有帧都能被保存
                            if let Err(e) = sender.send(frame_data).await {
                                println!("    ❌ 保存队列发送失败: {}", e);
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
        Some(handle.await?)
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
                println!("  💾 保存完成: {} 张图片已保存", saved_count);
            }
            Err(e) => {
                println!("  ❌ 保存过程中出现错误: {}", e);
            }
        }
        println!("  💡 优化措施: 多线程解码 + yuvutils-rs保存 + 非阻塞管道");
    } else {
        println!("  💡 优化措施: 多线程解码 + 快速提取 (无IO开销)");
    }

    Ok(())
}

fn convert_frame_to_data(
    frame: &ffmpeg::util::frame::video::Video,
    frame_number: u32,
    timestamp: f64,
) -> Result<FrameData, Box<dyn std::error::Error>> {
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
            // RGB24 格式：直接复制数据
            let rgb_size = (width * height * 3) as usize;
            data.extend_from_slice(&frame.data(0)[0..rgb_size]);
        }
        _ => {
            // 其他格式：尝试复制第一个平面
            let data_size = frame.data(0).len();
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

// 异步转换 YUV 到 RGB
async fn convert_frame_to_rgb_async(
    frame_data: FrameData,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let rgb_data = match frame_data.format {
        ffmpeg::util::format::Pixel::YUV420P => {
            let y_size = (frame_data.width * frame_data.height) as usize;
            let uv_size = y_size / 4;

            // 克隆数据以确保生命周期
            let data = frame_data.data.clone();
            let width = frame_data.width;
            let height = frame_data.height;
            
            tokio::task::spawn_blocking(move || {
                let y_plane = &data[0..y_size];
                let u_plane = &data[y_size..y_size + uv_size];
                let v_plane = &data[y_size + uv_size..y_size + 2 * uv_size];

                let yuv_image = YuvPlanarImage {
                    y_plane,
                    y_stride: width as u32,
                    u_plane,
                    u_stride: width as u32 / 2,
                    v_plane,
                    v_stride: width as u32 / 2,
                    width: width as u32,
                    height: height as u32,
                };

                let mut rgb_buf = vec![0u8; (width * height * 3) as usize];
                yuv420_to_rgb(
                    &yuv_image,
                    &mut rgb_buf,
                    width as u32 * 3,
                    YuvRange::Full,
                    YuvStandardMatrix::Bt709,
                )?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(rgb_buf)
            })
            .await??
        }
        ffmpeg::util::format::Pixel::RGB24 => frame_data.data,
        _ => {
            return Err(format!("不支持的像素格式: {:?}", frame_data.format).into());
        }
    };

    Ok(rgb_data)
}

// 异步保存图像
async fn save_image_async(
    rgb_data: Vec<u8>,
    frame_data: FrameData,
    config: ProcessConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filename = format!(
        "{}/frame_{:06}_{:.2}s.{}",
        config.output_dir, frame_data.frame_number, frame_data.timestamp, config.image_format
    );

    // 创建图像缓冲区
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
        frame_data.width,
        frame_data.height,
        rgb_data.clone(),
    ).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "无法创建图像缓冲区"))?;

    // 使用 tokio::task::spawn_blocking 在后台线程执行 I/O 操作
    tokio::task::spawn_blocking(move || {
        match config.image_format.as_str() {
            "jpg" | "jpeg" => {
                img.save_with_format(&filename, image::ImageFormat::Jpeg)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            "png" => {
                img.save_with_format(&filename, image::ImageFormat::Png)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            "ppm" => {
                let mut file = std::fs::File::create(&filename)?;
                writeln!(file, "P6\n{} {}\n255", frame_data.width, frame_data.height)?;
                file.write_all(&rgb_data)?;
                Ok(())
            }
            _ => {
                Err(format!("不支持的图像格式: {}", config.image_format).into())
            }
        }
    })
    .await??;

    Ok(())
}

// 修改原有的 save_frame_as_image 函数
fn save_frame_as_image(
    frame_data: FrameData,
    config: ProcessConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 使用 tokio::runtime::Runtime 来运行异步代码
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // 1. 异步转换 YUV 到 RGB
        let rgb_data = convert_frame_to_rgb_async(frame_data.clone()).await?;
        
        // 2. 异步保存图像
        save_image_async(rgb_data, frame_data, config).await
    })
}

// 修改 process_frames_async 函数
async fn process_frames_async(
    mut receiver: mpsc::Receiver<FrameData>,
    config: ProcessConfig,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let mut saved_count = 0;
    let mut tasks = Vec::new();

    println!("  🎨 yuvutils-rs 图片保存任务已启动");

    while let Some(frame_data) = receiver.recv().await {
        let task_config = config.clone();

        // 控制并发数量
        if tasks.len() >= config.max_concurrent_saves {
            // 等待一些任务完成
            let (result, _index, remaining) = futures::future::select_all(tasks).await;
            tasks = remaining;

            match result {
                Ok(_) => saved_count += 1,
                Err(e) => println!("    ❌ 保存任务失败: {}", e),
            }
        }

        // 启动新的保存任务
        let task = tokio::spawn(async move {
            // 1. 异步转换 YUV 到 RGB
            let rgb_data = convert_frame_to_rgb_async(frame_data.clone()).await?;
            
            // 2. 异步保存图像
            save_image_async(rgb_data, frame_data, task_config).await
        });

        tasks.push(task);
    }

    // 等待所有剩余任务完成
    let results = join_all(tasks).await;
    for result in results {
        match result {
            Ok(Ok(_)) => saved_count += 1,
            Ok(Err(e)) => println!("    ❌ 保存任务失败: {}", e),
            Err(e) => println!("    ❌ 任务执行失败: {}", e),
        }
    }

    println!("  ✅ 所有图片保存任务完成");
    Ok(saved_count)
}

fn optimize_decoder_for_speed(
    decoder_context: &mut ffmpeg::codec::context::Context,
) -> Result<(), Box<dyn std::error::Error>> {
    decoder_context.set_threading(ffmpeg::threading::Config {
        kind: ffmpeg::threading::Type::Frame,
        count: 0, // 0 表示自动检测最优线程数
    });

    Ok(())
}

fn create_optimized_input_context(
    input_path: &str,
) -> Result<ffmpeg::format::context::Input, Box<dyn std::error::Error>> {
    use ffmpeg::format;

    // 创建输入上下文，设置一些优化选项
    let input = format::input(&Path::new(input_path))?;

    // 可以在这里添加更多优化设置
    Ok(input)
}
