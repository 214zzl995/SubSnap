use anyhow::Result;
use std::path::Path;
use crate::converters::ConverterFactory;
use crate::cli::Cli;

pub fn list_available_modes() {
    println!("\n📋 可用的转换器:");
    
    let available = ConverterFactory::available_modes();
    for mode in available {
        println!("  ✅ {}: {}", mode.as_str(), mode.description());
    }

    println!("\n📋 可用的解码器:");
    
    let available_decoders = crate::decoders::DecoderFactory::available_decoders();
    for decoder in available_decoders {
        println!("  ✅ {}: {}", decoder.as_str(), decoder.description());
    }

    println!("\n💡 使用方法:");
    println!("  cargo run -- --converter ffmpeg --decoder ffmpeg    # 测试FFmpeg转换器 + FFmpeg解码器");
    println!("  cargo run -- --converter wgpu --decoder ffmpeg      # 测试WGPU转换器 + FFmpeg解码器");
    println!("  cargo run -- --converter yuvutils --decoder opencv  # 测试yuvutils转换器 + OpenCV解码器");
}

pub async fn show_help_and_demo(cli: &Cli) -> Result<()> {
    println!("\n🎯 SubSnap YUV到RGB转换测试工具");
    println!("✨ 支持多种不同的转换实现方式:");
    
    list_available_modes();
    
    println!("\n🚀 快速开始:");
    println!("  1. 测试转换器和解码器组合:");
    println!("     cargo run -- --converter ffmpeg --decoder ffmpeg --frames 5");
    println!("  2. 保存转换后的图片:");
    println!("     cargo run -- --converter ffmpeg --decoder ffmpeg --save-images");
    println!("  3. 测试不同组合:");
    println!("     cargo run -- --converter wgpu --decoder opencv --frames 10");

    if Path::new(&cli.input).exists() {
        println!("\n📁 检测到输入文件: {}", cli.input);
        println!("💡 可以运行: cargo run -- --converter ffmpeg --decoder ffmpeg --frames 3");
    } else {
        println!("\n⚠️  输入文件 {} 不存在", cli.input);
        println!("💡 请将测试视频文件命名为 input.mp4 或使用 --input 指定文件路径");
    }

    Ok(())
} 