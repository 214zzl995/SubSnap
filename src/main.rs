use anyhow::Result;
use clap::{Parser, ValueEnum};
use ffmpeg_next as ffmpeg;
use std::path::Path;

mod converter;
mod converters;

#[cfg(feature = "wgpu-mode")]
mod wgpu_processor;

use converter::*;

#[derive(Parser)]
#[command(name = "sub-snap")]
#[command(about = "SubSnap - 多模式YUV到RGB转换性能测试工具")]
#[command(version = "0.1.0")]
struct Cli {
    /// 转换模式
    #[arg(short, long, value_enum)]
    mode: Option<ConversionModeArg>,

    /// 是否运行所有模式的性能对比测试
    #[arg(short, long)]
    benchmark: bool,

    /// 输入视频文件路径
    #[arg(short, long, default_value = "input.mp4")]
    input: String,

    /// 要提取的帧数（用于测试）
    #[arg(short, long, default_value = "10")]
    frames: u32,

    /// 是否保存转换后的图片
    #[arg(short, long)]
    save_images: bool,

    /// 输出目录
    #[arg(short, long, default_value = "extracted_frames")]
    output: String,

    /// 列出可用的转换模式
    #[arg(short, long)]
    list_modes: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ConversionModeArg {
    /// 使用FFmpeg SWScale进行CPU转换
    Ffmpeg,
    /// 使用OpenCV库进行CPU转换
    Opencv,
    /// 使用手工实现进行CPU转换
    Manual,
    /// 使用WGPU进行GPU加速转换
    Wgpu,
    /// 使用yuvutils-rs进行高性能CPU转换
    Yuvutils,
}

impl From<ConversionModeArg> for ConversionMode {
    fn from(arg: ConversionModeArg) -> Self {
        match arg {
            ConversionModeArg::Ffmpeg => ConversionMode::Ffmpeg,
            ConversionModeArg::Opencv => ConversionMode::Opencv,
            ConversionModeArg::Manual => ConversionMode::Manual,
            ConversionModeArg::Wgpu => ConversionMode::Wgpu,
            ConversionModeArg::Yuvutils => ConversionMode::Yuvutils,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化 FFmpeg
    ffmpeg::init()?;

    println!("🎬 SubSnap - 多模式YUV到RGB转换测试工具");
    println!("📦 FFmpeg 初始化成功!");

    if cli.list_modes {
        list_available_modes();
        return Ok(());
    }

    if cli.benchmark {
        run_benchmark(&cli).await?;
    } else if let Some(mode) = cli.mode {
        run_single_mode(mode.into(), &cli).await?;
    } else {
        show_help_and_demo(&cli).await?;
    }

    Ok(())
}

fn list_available_modes() {
    println!("\n📋 可用的转换模式:");
    
    let available = ConverterFactory::available_modes();
    for mode in available {
        println!("  ✅ {}: {}", mode.as_str(), mode.description());
    }

    println!("\n💡 使用方法:");
    println!("  cargo run -- --mode ffmpeg     # 测试FFmpeg模式");
    #[cfg(feature = "wgpu-mode")]
    println!("  cargo run --features wgpu-mode -- --mode wgpu  # 测试WGPU模式");
    #[cfg(feature = "yuvutils-mode")]
    println!("  cargo run --features yuvutils-mode -- --mode yuvutils  # 测试yuvutils模式");
    println!("  cargo run --features all-modes -- --benchmark  # 运行所有模式的性能对比");
}

async fn show_help_and_demo(cli: &Cli) -> Result<()> {
    println!("\n🎯 SubSnap YUV到RGB转换测试工具");
    println!("✨ 支持三种不同的转换实现方式:");
    
    list_available_modes();
    
    println!("\n🚀 快速开始:");
    println!("  1. 测试单个模式:");
    println!("     cargo run -- --mode ffmpeg --frames 5");
    println!("  2. 运行性能对比:");
    println!("     cargo run --features all-modes -- --benchmark");
    println!("  3. 保存转换后的图片:");
    println!("     cargo run -- --mode ffmpeg --save-images");

    // 如果有有效的输入文件，演示一下基本功能
    if Path::new(&cli.input).exists() {
        println!("\n📁 检测到输入文件: {}", cli.input);
        println!("💡 可以运行: cargo run -- --mode ffmpeg --frames 3");
    } else {
        println!("\n⚠️  输入文件 {} 不存在", cli.input);
        println!("💡 请将测试视频文件命名为 input.mp4 或使用 --input 指定文件路径");
    }

    Ok(())
}

async fn run_single_mode(mode: ConversionMode, cli: &Cli) -> Result<()> {
    println!("\n🚀 测试 {} 模式...", mode.description());
    
    if !Path::new(&cli.input).exists() {
        anyhow::bail!("输入文件不存在: {}", cli.input);
    }

    let frames = extract_test_frames(&cli.input, cli.frames).await?;
    println!("📸 提取了 {} 帧用于测试", frames.len());

    let mut benchmark = Benchmark::new();
    benchmark.run_conversion_test(mode, &frames).await?;

    if cli.save_images {
        save_converted_images(mode, &frames, &cli.output).await?;
    }

    Ok(())
}

async fn run_benchmark(cli: &Cli) -> Result<()> {
    println!("\n🏆 运行性能基准测试...");

    if !Path::new(&cli.input).exists() {
        anyhow::bail!("输入文件不存在: {}", cli.input);
    }

    let frames = extract_test_frames(&cli.input, cli.frames).await?;
    println!("📸 提取了 {} 帧用于性能测试", frames.len());

    let mut benchmark = Benchmark::new();
    let available_modes = ConverterFactory::available_modes();

    for mode in available_modes {
        match benchmark.run_conversion_test(mode, &frames).await {
            Ok(_) => {}
            Err(e) => {
                println!("❌ {} 模式测试失败: {}", mode.as_str(), e);
            }
        }
        println!(); // 分隔线
    }

    benchmark.print_comparison();

    if cli.save_images {
        println!("\n💾 保存最佳模式的转换结果...");
        if let Some((&best_mode, _)) = benchmark.stats.iter()
            .max_by(|a, b| a.1.fps.partial_cmp(&b.1.fps).unwrap_or(std::cmp::Ordering::Equal)) {
            save_converted_images(best_mode, &frames, &cli.output).await?;
        }
    }

    Ok(())
}

async fn extract_test_frames(input_path: &str, max_frames: u32) -> Result<Vec<FrameData>> {
    use ffmpeg::media;

    let mut input = ffmpeg::format::input(&Path::new(input_path))?;
    
    // 找到视频流
    let video_stream_index = input.streams()
        .enumerate()
        .find(|(_, stream)| stream.parameters().medium() == media::Type::Video)
        .map(|(i, _)| i)
        .ok_or_else(|| anyhow::anyhow!("未找到视频流"))?;

    let stream = input.streams().nth(video_stream_index).unwrap();
    let decoder_context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
    let mut decoder = decoder_context.decoder().video()?;

    let mut frames = Vec::new();
    let mut frame_count = 0;

    for (stream, packet) in input.packets() {
        if stream.index() == video_stream_index && frame_count < max_frames {
            decoder.send_packet(&packet)?;

            let mut decoded = ffmpeg::util::frame::video::Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() && frame_count < max_frames {
                let timestamp = decoded.timestamp().unwrap_or(0) as f64 * f64::from(stream.time_base());
                
                // 正确提取YUV420P格式的帧数据
                let mut frame_data = Vec::new();
                
                if decoded.format() == ffmpeg::util::format::Pixel::YUV420P {
                    let width = decoded.width() as usize;
                    let height = decoded.height() as usize;
                    
                    // Y平面
                    let y_plane = decoded.data(0);
                    let y_stride = decoded.stride(0) as usize;
                    for y in 0..height {
                        let start = y * y_stride;
                        let end = start + width;
                        frame_data.extend_from_slice(&y_plane[start..end]);
                    }
                    
                    // U平面
                    let u_plane = decoded.data(1);
                    let u_stride = decoded.stride(1) as usize;
                    let uv_width = width / 2;
                    let uv_height = height / 2;
                    for y in 0..uv_height {
                        let start = y * u_stride;
                        let end = start + uv_width;
                        frame_data.extend_from_slice(&u_plane[start..end]);
                    }
                    
                    // V平面
                    let v_plane = decoded.data(2);
                    let v_stride = decoded.stride(2) as usize;
                    for y in 0..uv_height {
                        let start = y * v_stride;
                        let end = start + uv_width;
                        frame_data.extend_from_slice(&v_plane[start..end]);
                    }
                } else {
                    // 对于非YUV420P格式，复制第一个平面
                    let data_size = decoded.data(0).len();
                    frame_data = vec![0u8; data_size];
                    frame_data.copy_from_slice(decoded.data(0));
                }

                let frame = FrameData {
                    frame_number: frame_count + 1,
                    timestamp,
                    width: decoded.width(),
                    height: decoded.height(),
                    data: frame_data,
                    format: decoded.format(),
                };

                if frame_count < 3 {
                    println!(
                        "  📸 提取帧#{}: {}x{}, 格式: {:?}, 时间戳: {:.2}s",
                        frame.frame_number,
                        frame.width,
                        frame.height,
                        frame.format,
                        frame.timestamp
                    );
                }

                frames.push(frame);
                frame_count += 1;
            }
        }
    }

    Ok(frames)
}

async fn save_converted_images(
    mode: ConversionMode,
    frames: &[FrameData],
    output_dir: &str,
) -> Result<()> {
    use std::fs;
    use image::{ImageBuffer, Rgb};

    println!("💾 使用 {} 模式保存转换后的图片到 {}...", mode.as_str(), output_dir);
    
    fs::create_dir_all(output_dir)?;
    
    let mut converter = ConverterFactory::create_converter(mode).await?;
    
    for frame in frames.iter().take(5) { // 只保存前5帧作为示例
        match converter.convert(frame).await {
            Ok(rgb_data) => {
                let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
                    frame.width,
                    frame.height,
                    rgb_data,
                ).ok_or_else(|| anyhow::anyhow!("无法创建图像缓冲区"))?;

                let filename = format!(
                    "{}/frame_{}_{:04}.jpg",
                    output_dir,
                    mode.as_str(),
                    frame.frame_number
                );
                img.save(&filename)?;
                
                println!(
                    "  ✅ 保存: {} ({}x{})",
                    filename,
                    frame.width,
                    frame.height
                );
            }
            Err(e) => {
                println!("  ❌ 转换帧#{} 失败: {}", frame.frame_number, e);
            }
        }
    }

    converter.cleanup().await?;
    Ok(())
}
