use anyhow::Result;
use std::path::Path;
use crate::converters::ConverterFactory;
use crate::cli::Cli;

pub fn list_available_modes() {
    println!("\n📋 可用的转换模式:");
    
    let available = ConverterFactory::available_modes();
    for mode in available {
        println!("  ✅ {}: {}", mode.as_str(), mode.description());
    }

    println!("\n💡 使用方法:");
    println!("  cargo run -- --mode ffmpeg     # 测试FFmpeg模式");
    println!("  cargo run -- --mode wgpu      # 测试WGPU模式");
    println!("  cargo run -- --mode yuvutils  # 测试yuvutils模式");
    println!("  cargo run -- --benchmark      # 运行所有模式的性能对比");
}

pub async fn show_help_and_demo(cli: &Cli) -> Result<()> {
    println!("\n🎯 SubSnap YUV到RGB转换测试工具");
    println!("✨ 支持多种不同的转换实现方式:");
    
    list_available_modes();
    
    println!("\n🚀 快速开始:");
    println!("  1. 测试单个模式:");
    println!("     cargo run -- --mode ffmpeg --frames 5");
    println!("  2. 运行性能对比:");
    println!("     cargo run -- --benchmark");
    println!("  3. 保存转换后的图片:");
    println!("     cargo run -- --mode ffmpeg --save-images");

    if Path::new(&cli.input).exists() {
        println!("\n📁 检测到输入文件: {}", cli.input);
        println!("💡 可以运行: cargo run -- --mode ffmpeg --frames 3");
    } else {
        println!("\n⚠️  输入文件 {} 不存在", cli.input);
        println!("💡 请将测试视频文件命名为 input.mp4 或使用 --input 指定文件路径");
    }

    Ok(())
} 