use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConversionMode {
    FFmpeg,
    OpenCV, 
    Manual,
    WGPU,
    Yuvutils,
}

impl ConversionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversionMode::FFmpeg => "ffmpeg",
            ConversionMode::OpenCV => "opencv",
            ConversionMode::Manual => "manual",
            ConversionMode::WGPU => "wgpu",
            ConversionMode::Yuvutils => "yuvutils",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ConversionMode::FFmpeg => "使用FFmpeg SWScale进行CPU转换",
            ConversionMode::OpenCV => "使用OpenCV库进行CPU转换",
            ConversionMode::Manual => "手动实现YUV420P到RGB转换",
            ConversionMode::WGPU => "使用WGPU进行GPU加速转换",
            ConversionMode::Yuvutils => "使用yuvutils库进行SIMD优化转换",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameData {
    pub frame_number: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ffmpeg_next::util::format::Pixel,
}

#[derive(Debug, Clone)]
pub struct ChannelFrameData {
    pub frame_number: u32,
    pub width: u32,
    pub height: u32,
    pub yuv_data: Vec<u8>,
    pub format: ffmpeg_next::util::format::Pixel,
}

impl From<FrameData> for ChannelFrameData {
    fn from(frame_data: FrameData) -> Self {
        Self {
            frame_number: frame_data.frame_number,
            width: frame_data.width,
            height: frame_data.height,
            yuv_data: frame_data.data,
            format: frame_data.format,
        }
    }
}

impl From<ChannelFrameData> for FrameData {
    fn from(channel_frame_data: ChannelFrameData) -> Self {
        Self {
            frame_number: channel_frame_data.frame_number,
            width: channel_frame_data.width,
            height: channel_frame_data.height,
            data: channel_frame_data.yuv_data,
            format: channel_frame_data.format,
        }
    }
}

pub async fn process_frame_with_mode(
    mut receiver: tokio::sync::mpsc::Receiver<ChannelFrameData>,
    mode: ConversionMode,
    output_dir: Option<String>,
) -> Result<u32> {
    use std::fs;
    use image::{ImageBuffer, Rgb};
    
    if let Some(ref output_dir) = output_dir {
        fs::create_dir_all(output_dir)?;
    }
    
    let mut converter = ConverterFactory::create_converter(mode).await?;
    let mut processed_count = 0u32;
    
    while let Some(channel_frame) = receiver.recv().await {
        let frame_data: FrameData = channel_frame.into();
        
        match converter.convert(&frame_data).await {
            Ok(rgb_data) => {
                if let Some(ref output_dir) = output_dir {
                    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
                        frame_data.width,
                        frame_data.height,
                        rgb_data,
                    ).ok_or_else(|| anyhow::anyhow!("无法创建图像缓冲区"))?;

                    let filename = format!(
                        "{}/frame_{}_{:04}.jpg",
                        output_dir,
                        mode.as_str(),
                        frame_data.frame_number
                    );
                    
                    img.save(&filename)?;
                }
                processed_count += 1;
            }
            Err(e) => {
                eprintln!("转换帧#{} 失败: {}", frame_data.frame_number, e);
            }
        }
    }
    
    converter.cleanup().await?;
    Ok(processed_count)
}

#[async_trait::async_trait(?Send)]
pub trait YuvToRgbConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>>;
    async fn cleanup(&mut self) -> Result<()> { Ok(()) }
}

pub struct ConverterFactory;

impl ConverterFactory {
    pub async fn create_converter(mode: ConversionMode) -> Result<Box<dyn YuvToRgbConverter>> {
        match mode {
            ConversionMode::FFmpeg => {
                Ok(Box::new(crate::converters::ffmpeg_converter::FfmpegConverter::new()))
            }
            ConversionMode::OpenCV => {
                Ok(Box::new(crate::converters::opencv_converter::OpencvConverter::new()))
            }
            ConversionMode::Manual => {
                Ok(Box::new(crate::converters::manual_converter::ManualConverter::new()))
            }
            ConversionMode::WGPU => {
                Ok(Box::new(crate::converters::wgpu_converter::WgpuConverter::new().await?))
            }
            ConversionMode::Yuvutils => {
                Ok(Box::new(crate::converters::yuvutils_converter::YuvutilsConverter::new()))
            }
        }
    }

    pub fn available_modes() -> Vec<ConversionMode> {
        vec![
            ConversionMode::FFmpeg,
            ConversionMode::OpenCV,
            ConversionMode::Manual,
            ConversionMode::WGPU,
            ConversionMode::Yuvutils,
        ]
    }
}

pub mod ffmpeg_converter;
pub mod opencv_converter;
pub mod manual_converter;
pub mod wgpu_converter;
pub mod yuvutils_converter; 