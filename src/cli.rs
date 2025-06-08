use clap::{Parser, ValueEnum};
use crate::converters::ConversionMode;

#[derive(Parser)]
#[command(name = "sub-snap")]
#[command(about = "SubSnap - 多模式YUV到RGB转换性能测试工具")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// 转换模式
    #[arg(short, long, value_enum)]
    pub mode: Option<ConversionModeArg>,

    /// 是否运行所有模式的性能对比测试
    #[arg(short, long)]
    pub benchmark: bool,

    /// 输入视频文件路径
    #[arg(short, long, default_value = "input.mp4")]
    pub input: String,

    /// 要提取的帧数（用于测试，0 表示提取所有采样帧）
    #[arg(short, long, default_value = "0")]
    pub frames: u32,

    /// 每秒采样帧数（1表示每秒1帧，0表示提取所有原始帧）
    #[arg(long, default_value = "1")]
    pub fps: u32,

    /// 是否保存转换后的图片
    #[arg(short, long)]
    pub save_images: bool,

    /// 输出目录
    #[arg(short, long, default_value = "extracted_frames")]
    pub output: String,

    /// 列出可用的转换模式
    #[arg(short, long)]
    pub list_modes: bool,

    /// 显示详细的FFmpeg日志信息
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ConversionModeArg {
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
            ConversionModeArg::Ffmpeg => ConversionMode::FFmpeg,
            ConversionModeArg::Opencv => ConversionMode::OpenCV,
            ConversionModeArg::Manual => ConversionMode::Manual,
            ConversionModeArg::Wgpu => ConversionMode::WGPU,
            ConversionModeArg::Yuvutils => ConversionMode::Yuvutils,
        }
    }
} 