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