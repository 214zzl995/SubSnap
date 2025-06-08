use anyhow::Result;
use clap::Parser;
use ffmpeg_next as ffmpeg;

mod cli;
mod converters;
mod wgpu_processor;
mod frame_extraction;
mod benchmark;
mod app;

use cli::Cli;
use app::{list_available_modes, show_help_and_demo};
use benchmark::{run_benchmark, run_single_mode};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化 FFmpeg
    ffmpeg::init()?;
    
    // 根据verbose参数设置FFmpeg日志级别
    if cli.verbose {
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Info);
    } else {
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Error);
    }

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


