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

    let input_path = cli.input.clone();
    let frames = cli.frames;
    let fps = cli.fps;
    let save_images = cli.save_images;
    let output_path = cli.output.clone();
    
    let (sender, receiver) = tokio::sync::mpsc::channel::<crate::converters::ChannelFrameData>(100);
    
    let output_dir = if save_images { 
        Some(output_path) 
    } else { 
        None 
    };
    
    let convert_task = tokio::task::spawn_local(async move {
        crate::converters::process_frame_with_mode(receiver, mode, output_dir).await
    });
    
    let extract_task = tokio::task::spawn_local(async move {
        crate::frame_extraction::extract_frames_streaming(
            &input_path,
            frames,
            fps,
            sender,
        ).await
    });
    
    let (extract_result, converted_frames) = tokio::try_join!(
        async { extract_task.await.map_err(|e| anyhow::anyhow!("Extract task failed: {}", e))? },
        async { convert_task.await.map_err(|e| anyhow::anyhow!("Convert task failed: {}", e))? }
    )?;
    
    println!("处理完成：提取 {} 帧，转换 {} 帧，耗时 {:.2}秒", 
             extract_result.frames_processed, 
             converted_frames,
             extract_result.total_duration.as_secs_f64());

    Ok(())
} 