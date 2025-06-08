use anyhow::Result;
use std::path::Path;
use crate::converters::ConversionMode;
use crate::cli::Cli;

pub async fn run_benchmark(_cli: &Cli) -> Result<()> {
    println!("基准测试功能已移除，请使用单模式测试");
    Ok(())
}

pub async fn run_single_mode(mode: ConversionMode, cli: &Cli) -> Result<()> {
    if !Path::new(&cli.input).exists() {
        anyhow::bail!("输入文件不存在: {}", cli.input);
    }

    let result = crate::frame_extraction::extract_convert_save_streaming(
        &cli.input, 
        cli.frames, 
        cli.fps, 
        mode, 
        if cli.save_images { Some(&cli.output) } else { None }
    ).await?;

    println!("处理完成：提取 {} 帧，耗时 {:.2}秒", 
             result.frames_processed, 
             result.total_duration.as_secs_f64());

    Ok(())
} 