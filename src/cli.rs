use clap::Parser;

#[derive(Parser)]
#[command(name = "sub-snap")]
#[command(about = "SubSnap - 多模式YUV到RGB转换性能测试工具")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// 转换器类型
    #[arg(short = 'c', long = "converter", value_enum)]
    pub converter: Option<crate::converters::ConversionMode>,

    /// 输入视频文件路径
    #[arg(short, long, default_value = "input1.mp4")]
    pub input: String,

    /// 要提取的帧数（用于测试，0 表示提取所有采样帧）
    #[arg(short, long, default_value = "0")]
    pub frames: u32,

    /// 每秒采样帧数（1表示每秒1帧，0表示提取所有原始帧）
    #[arg(long, default_value = "1")]
    pub fps: u32,

    /// 解码器类型
    #[arg(short, long, value_enum, default_value = "ffmpeg")]
    pub decoder: crate::decoders::DecoderType,

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

 