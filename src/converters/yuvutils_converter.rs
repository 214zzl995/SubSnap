use anyhow::Result;
use crate::converter::{YuvToRgbConverter, FrameData, ConversionMode};

/// YuvUtils-rs高性能转换器
/// 
/// 使用yuvutils-rs库进行SIMD优化的YUV到RGB转换
/// 专门针对YUV420P格式优化，提供纯Rust的高性能实现
pub struct YuvutilsConverter;

impl YuvutilsConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for YuvutilsConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        use yuvutils_rs::*;
        
        if frame_data.format != ffmpeg_next::util::format::Pixel::YUV420P {
            anyhow::bail!("Yuvutils converter only supports YUV420P format");
        }
        
        let width = frame_data.width;
        let height = frame_data.height;
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        
        if frame_data.data.len() < y_size + 2 * uv_size {
            anyhow::bail!("Invalid YUV data size");
        }
        
        let y_plane = &frame_data.data[0..y_size];
        let u_plane = &frame_data.data[y_size..y_size + uv_size];
        let v_plane = &frame_data.data[y_size + uv_size..y_size + 2 * uv_size];
        
        let mut rgb_data = vec![0u8; (width * height * 3) as usize];
        
        let yuv_image = YuvPlanarImage {
            y_plane,
            u_plane,
            v_plane,
            width,
            height,
            y_stride: width,
            u_stride: width / 2,
            v_stride: width / 2,
        };
        
        yuv420_to_rgb(
            &yuv_image,
            &mut rgb_data,
            width * 3,
            YuvRange::Limited,
            YuvStandardMatrix::Bt709,
        )?;
        
        Ok(rgb_data)
    }

    fn get_mode(&self) -> ConversionMode {
        ConversionMode::Yuvutils
    }
} 