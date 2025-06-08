use anyhow::Result;
use crate::converters::{YuvToRgbConverter, FrameData};

/// OpenCV库转换器
/// 
/// 使用OpenCV库的cvtColor函数进行YUV到RGB转换
/// 利用OpenCV优化的色彩空间转换算法
pub struct OpencvConverter;

impl OpencvConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for OpencvConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        if frame_data.format != ffmpeg_next::util::format::Pixel::YUV420P {
            anyhow::bail!("OpenCV converter only supports YUV420P format");
        }
        
        let width = frame_data.width as i32;
        let height = frame_data.height as i32;
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        
        if frame_data.data.len() < y_size + 2 * uv_size {
            anyhow::bail!("Invalid YUV data size: expected {}, got {}", 
                         y_size + 2 * uv_size, frame_data.data.len());
        }
        
        // 使用真正的OpenCV cvt_color进行YUV420P到RGB转换
        use opencv::core::*;
        use opencv::imgproc::*;
        
        // 创建连续的YUV数据Mat
        let yuv_data = Mat::from_slice(&frame_data.data)?;
        let yuv_mat = yuv_data.reshape(1, height * 3 / 2)?;
        
        // 使用OpenCV的COLOR_YUV2RGB_I420转换
        let mut rgb_mat = Mat::default();
        cvt_color(
            &yuv_mat,
            &mut rgb_mat,
            COLOR_YUV2RGB_I420,
            0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
        
        // 提取RGB数据
        let rgb_size = (width * height * 3) as usize;
        let rgb_data = rgb_mat.data_bytes()?.to_vec();
        
        if rgb_data.len() >= rgb_size {
            Ok(rgb_data[0..rgb_size].to_vec())
        } else {
            anyhow::bail!("OpenCV conversion resulted in insufficient data: expected {}, got {}", 
                         rgb_size, rgb_data.len());
        }
    }

} 