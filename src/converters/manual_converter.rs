use anyhow::Result;
use crate::converter::{YuvToRgbConverter, FrameData, ConversionMode};

/// 手工实现的YUV转换器
/// 
/// 使用手工实现的YUV420P到RGB转换算法
/// 主要用于教育目的和理解转换原理
pub struct ManualConverter;

impl ManualConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for ManualConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        if frame_data.format != ffmpeg_next::util::format::Pixel::YUV420P {
            anyhow::bail!("Manual converter only supports YUV420P format");
        }
        
        let width = frame_data.width as i32;
        let height = frame_data.height as i32;
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        
        if frame_data.data.len() < y_size + 2 * uv_size {
            anyhow::bail!("Invalid YUV data size");
        }
        
        // 简单的YUV420P到RGB转换（手工实现）
        let y_plane = &frame_data.data[0..y_size];
        let u_plane = &frame_data.data[y_size..y_size + uv_size];
        let v_plane = &frame_data.data[y_size + uv_size..y_size + 2 * uv_size];
        
        let mut rgb_data = vec![0u8; (width * height * 3) as usize];
        
        for y in 0..height {
            for x in 0..width {
                let y_idx = (y * width + x) as usize;
                let uv_idx = ((y / 2) * (width / 2) + (x / 2)) as usize;
                
                let y_val = y_plane[y_idx] as f32;
                let u_val = u_plane[uv_idx] as f32 - 128.0;
                let v_val = v_plane[uv_idx] as f32 - 128.0;
                
                // YUV到RGB转换公式 (ITU-R BT.601标准)
                let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
                let g = (y_val - 0.344136 * u_val - 0.714136 * v_val).clamp(0.0, 255.0) as u8;
                let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;
                
                let rgb_idx = (y * width + x) as usize * 3;
                rgb_data[rgb_idx] = r;
                rgb_data[rgb_idx + 1] = g;
                rgb_data[rgb_idx + 2] = b;
            }
        }
        
        Ok(rgb_data)
    }

    fn get_mode(&self) -> ConversionMode {
        ConversionMode::Manual
    }
} 