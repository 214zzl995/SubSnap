use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum ConversionMode {
    /// 使用FFmpeg SWScale进行CPU转换
    #[value(name = "ffmpeg")]
    FFmpeg,
    /// 使用OpenCV库进行CPU转换
    #[value(name = "opencv")]
    OpenCV, 
    /// 使用手工实现进行CPU转换
    #[value(name = "manual")]
    Manual,
    /// 使用WGPU进行GPU加速转换
    #[value(name = "wgpu")]
    WGPU,
    /// 使用yuvutils-rs进行高性能CPU转换
    #[value(name = "yuvutils")]
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
    
    let mut processed_count = 0u32;
    
    // 特殊处理WGPU模式 - 使用批处理
    if mode == ConversionMode::WGPU {
        let mut frame_batch = Vec::new();
        
        const TARGET_BATCH_SIZE: usize = 64;
        
        let mut current_batch_size = 0;
        
        while let Some(channel_frame) = receiver.recv().await {
            let frame_data: FrameData = channel_frame.into();
            
            // 设置批次大小
            if current_batch_size == 0 {
                current_batch_size = TARGET_BATCH_SIZE;
            }
            
            frame_batch.push(frame_data);
            
            // 当批次满了时处理批次
            if frame_batch.len() >= current_batch_size {
                let batch_results = process_frame_batch(&frame_batch, &output_dir, &mode).await?;
                processed_count += batch_results;
                frame_batch.clear();
            }
        }
        
        // 处理剩余的帧
        if !frame_batch.is_empty() {
            let batch_results = process_frame_batch(&frame_batch, &output_dir, &mode).await?;
            processed_count += batch_results;
        }
    } else {
        // 原始逐帧处理逻辑
        let mut converter = ConverterFactory::create_converter(mode).await?;
        
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
    }
    
    Ok(processed_count)
}

/// 处理一个批次的帧
async fn process_frame_batch(
    frame_batch: &[FrameData],
    output_dir: &Option<String>,
    mode: &ConversionMode,
) -> Result<u32> {
    use image::{ImageBuffer, Rgb};
    
    if frame_batch.is_empty() {
        return Ok(0);
    }
    
    let mut processor = crate::converters::wgpu_converter::GpuImageProcessor::new().await?;
    
    let batch_data: Vec<(Vec<u8>, u32, u32)> = frame_batch
        .iter()
        .map(|frame| (frame.data.clone(), frame.width, frame.height))
        .collect();
    
    let batch_results = processor.convert_yuv420p_to_rgb(&batch_data).await?;
    
    for (frame_idx, rgb_data) in batch_results.iter().enumerate() {
        if let Some(ref output_dir) = output_dir {
            let frame = &frame_batch[frame_idx];
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
                frame.width,
                frame.height,
                rgb_data.clone(),
            ).ok_or_else(|| anyhow::anyhow!("无法创建图像缓冲区"))?;

            let filename = format!(
                "{}/frame_{}_{:04}.jpg",
                output_dir,
                mode.as_str(),
                frame.frame_number
            );
            
            img.save(&filename)?;
        }
    }
    
    Ok(batch_results.len() as u32)
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
                Ok(Box::new(crate::converters::wgpu_converter::WgpuBatchConverter::new(true, None, None).await?))
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