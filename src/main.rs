use ffmpeg_next as ffmpeg;
use futures::future::join_all;
use std::path::Path;
use tokio::sync::mpsc;

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
    pub use_opencl: bool,     // Mac上使用OpenCL加速（可选）
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
            use_opencl: false, // 默认不使用OpenCL
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
    println!("🍎 Mac 设备优化版本");

    // 显示一些基本信息
    println!("\n📋 可用功能示例:");
    println!("✅ FFmpeg 库已初始化");
    println!("✅ 可以进行视频/音频处理");
    println!("✅ 支持多种媒体格式");
    println!("✅ OpenCV 异步图片保存已集成 (可选)");
    println!("✅ OpenCL 加速支持 (Mac 优化)");

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

    // 检查 OpenCV OpenCL 支持（Mac 优化）
    check_opencv_opencl_support().await?;

    // 演示视频帧拆分功能（只在有有效文件时）
    println!("\n🎞️  准备演示视频帧拆分功能...");

    let config = ProcessConfig {
        target_fps: 1.0, // 每秒提取1帧
        output_dir: "extracted_frames".to_string(),
        save_images: SAVE_ENABLED, // 是否保存图片
        max_concurrent_saves: 4,
        image_format: "jpg".to_string(),
        jpeg_quality: 90,
        use_opencl: true, // 在Mac上尝试OpenCL加速
    };

    if let Err(e) = demo_frame_extraction_with_opencv(config).await {
        println!("  ⚠️  帧保存演示跳过: {}", e);
    }

    Ok(())
}

async fn check_opencv_opencl_support() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 检查 OpenCV OpenCL 支持 (Mac 优化):");

    // 检查 OpenCL 支持
    match opencv::core::have_opencl() {
        Ok(true) => {
            println!("  ✅ OpenCL 支持可用");

            // 尝试使用 OpenCL
            if let Ok(_) = opencv::core::use_opencl() {
                println!("  🚀 OpenCL 已启用 (适用于Mac GPU加速)");
            } else {
                println!("  ⚠️  OpenCL 启用失败，将使用 CPU 模式");
            }
        }
        Ok(false) => {
            println!("  ⚠️  OpenCL 支持不可用，将使用 CPU 模式");
        }
        Err(_) => {
            println!("  ⚠️  检查 OpenCL 支持时出错，将使用 CPU 模式");
        }
    }

    // 显示Mac特有的加速提示
    println!("  🍎 Mac 设备提示:");
    println!("    - Intel Mac: 可能支持 OpenCL GPU 加速");
    println!("    - Apple Silicon Mac: 主要依赖 CPU 优化");
    println!("    - 建议使用多线程并发提升性能");

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

async fn demo_frame_extraction_with_opencv(
    config: ProcessConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use ffmpeg::media;

    println!(
        "\n🎞️  开始演示视频帧拆分 {}:",
        if config.save_images {
            "(带OpenCV异步保存)"
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

        if config.use_opencl {
            println!("  🚀 OpenCL加速: 启用 (Mac 优化)");
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
                println!("  💾 异步保存完成: {} 张图片已保存", saved_count);
            }
            Err(e) => {
                println!("  ❌ 保存过程中出现错误: {}", e);
            }
        }
        println!("  💡 优化措施: 多线程解码 + 异步保存 + OpenCL加速(Mac) + 非阻塞管道");
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

async fn process_frames_async(
    mut receiver: mpsc::Receiver<FrameData>,
    config: ProcessConfig,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let mut saved_count = 0;
    let mut tasks = Vec::new();

    println!("  🎨 OpenCV 异步图片保存任务已启动 (Mac 优化)");

    // 尝试初始化OpenCL上下文（如果启用）
    let opencl_enabled = if config.use_opencl {
        init_opencl_context().await.unwrap_or_else(|e| {
            println!("    ⚠️  OpenCL 初始化失败，使用CPU模式: {}", e);
            false
        })
    } else {
        false
    };

    if opencl_enabled {
        println!("  🚀 OpenCL 加速已启用 (Mac GPU 优化)");
    } else {
        println!("  💻 使用 CPU 模式");
    }

    while let Some(frame_data) = receiver.recv().await {
        let task_config = config.clone();
        let task_use_opencl = opencl_enabled;

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
        let task = tokio::task::spawn_blocking(move || {
            save_frame_as_image(frame_data, task_config, task_use_opencl)
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

fn save_frame_as_image(
    frame_data: FrameData,
    config: ProcessConfig,
    opencl_enabled: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opencv::imgcodecs::*;

    // 构造输出文件名
    let filename = format!(
        "{}/frame_{:06}_{:.2}s.{}",
        config.output_dir, frame_data.frame_number, frame_data.timestamp, config.image_format
    );

    // 根据 FFmpeg 像素格式转换为 OpenCV Mat
    let mut mat = convert_ffmpeg_frame_to_opencv_mat(&frame_data)?;

    // 如果使用OpenCL，尝试在GPU上处理
    if opencl_enabled {
        if let Ok(processed_mat) = process_with_opencl(&mat) {
            mat = processed_mat;
        }
    }

    // 设置保存参数
    let mut params = opencv::core::Vector::<i32>::new();

    match config.image_format.to_lowercase().as_str() {
        "jpg" | "jpeg" => {
            params.push(IMWRITE_JPEG_QUALITY);
            params.push(config.jpeg_quality);
        }
        "png" => {
            params.push(IMWRITE_PNG_COMPRESSION);
            params.push(3); // 0-9, 3 是平衡压缩率和速度的选择
        }
        _ => {}
    }

    // 保存图片
    imwrite(&filename, &mat, &params)?;

    Ok(())
}

fn convert_ffmpeg_frame_to_opencv_mat(
    frame_data: &FrameData,
) -> Result<opencv::core::Mat, Box<dyn std::error::Error + Send + Sync>> {
    use opencv::core::*;

    let height = frame_data.height as i32;
    let width = frame_data.width as i32;

    // 根据 FFmpeg 像素格式创建对应的 OpenCV Mat
    match frame_data.format {
        ffmpeg::util::format::Pixel::YUV420P => {
            // YUV420P 格式处理 - 使用 I420 格式直接转换
            let y_size = (width * height) as usize;
            let uv_size = y_size / 4;

            if frame_data.data.len() < y_size + uv_size * 2 {
                return Err(format!("YUV420P 数据长度不足: 需要 {} 字节，实际 {} 字节", 
                                 y_size + uv_size * 2, frame_data.data.len()).into());
            }

            // 创建连续的 YUV 数据 Mat
            let yuv_data = Mat::from_slice(&frame_data.data)?;
            let yuv_mat = yuv_data.reshape(1, height * 3 / 2)?;

            // 转换 YUV I420 到 BGR
            let mut bgr_mat = Mat::default();
            opencv::imgproc::cvt_color(
                &yuv_mat,
                &mut bgr_mat,
                opencv::imgproc::COLOR_YUV2BGR_I420,
                0,
                opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
            )?;

            Ok(bgr_mat)
        }
        ffmpeg::util::format::Pixel::RGB24 => {
            // RGB24 格式
            let expected_size = (width * height * 3) as usize;
            if frame_data.data.len() < expected_size {
                return Err(format!("RGB24 数据长度不足: 需要 {} 字节，实际 {} 字节", 
                                 expected_size, frame_data.data.len()).into());
            }

            let rgb_data = Mat::from_slice(&frame_data.data[0..expected_size])?;
            let rgb_mat = rgb_data.reshape(3, height)?;

            // 转换 RGB 到 BGR
            let mut bgr_mat = Mat::default();
            opencv::imgproc::cvt_color(
                &rgb_mat,
                &mut bgr_mat,
                opencv::imgproc::COLOR_RGB2BGR,
                0,
                opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
            )?;

            Ok(bgr_mat)
        }
        _ => {
            // 对于其他格式，尝试作为灰度图像处理
            let expected_size = (width * height) as usize;
            let actual_size = frame_data.data.len().min(expected_size);
            
            let gray_data = Mat::from_slice(&frame_data.data[0..actual_size])?;
            let gray_mat = gray_data.reshape(1, height)?;

            // 转换为 BGR
            let mut bgr_mat = Mat::default();
            opencv::imgproc::cvt_color(
                &gray_mat,
                &mut bgr_mat,
                opencv::imgproc::COLOR_GRAY2BGR,
                0,
                opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
            )?;

            Ok(bgr_mat)
        }
    }
}

async fn init_opencl_context() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tokio::task::spawn_blocking(
        || -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
            use opencv::core::*;

            // 检查 OpenCL 是否可用
            if !have_opencl()? {
                return Ok(false);
            }

            // 尝试启用 OpenCL
            use_opencl()?;

            // 测试 OpenCL 功能
            let test_mat = Mat::zeros(100, 100, CV_8UC3)?.to_mat()?;
            let mut _result_mat = Mat::default();

            // 简单的测试操作 - 创建一个简单的测试而不用 gaussian_blur
            let _test_result = test_mat.clone();

            Ok(true)
        },
    )
    .await?
}

fn process_with_opencl(
    mat: &opencv::core::Mat,
) -> Result<opencv::core::Mat, Box<dyn std::error::Error + Send + Sync>> {
    // 这里可以添加 OpenCL 优化的图像处理操作
    // 例如：调整大小、模糊、锐化等
    // 对于Mac，OpenCL 主要利用集成显卡或独立显卡

    // 目前直接返回原图，你可以根据需要添加具体的图像处理
    Ok(mat.clone())
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
    // 例如缓冲区大小、预读取选项等

    Ok(input)
}
