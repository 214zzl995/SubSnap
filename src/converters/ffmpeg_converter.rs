use anyhow::Result;
use crate::converters::{YuvToRgbConverter, FrameData};

/// FFmpeg SWScale转换器
/// 
/// 使用FFmpeg的SWScale库进行高效的YUV到RGB转换
/// 这是最成熟和优化的实现，适合生产环境使用
pub struct FfmpegConverter {
    scaler: Option<ffmpeg_next::software::scaling::context::Context>,
}

impl FfmpegConverter {
    pub fn new() -> Self {
        Self {
            scaler: None,
        }
    }

    /// 确保scaler已初始化，如果没有则创建新的scaler
    fn ensure_scaler(&mut self, width: u32, height: u32, format: ffmpeg_next::util::format::Pixel) -> Result<()> {
        if self.scaler.is_none() {
            let scaler = ffmpeg_next::software::scaling::context::Context::get(
                format,
                width,
                height,
                ffmpeg_next::util::format::Pixel::RGB24,
                width,
                height,
                ffmpeg_next::software::scaling::Flags::BILINEAR,
            )?;
            self.scaler = Some(scaler);
        }
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for FfmpegConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        self.ensure_scaler(frame_data.width, frame_data.height, frame_data.format)?;
        
        // 创建输入帧
        let mut input_frame = ffmpeg_next::util::frame::video::Video::new(
            frame_data.format,
            frame_data.width,
            frame_data.height
        );
        
        // 为YUV420P格式正确设置三个平面的数据
        if frame_data.format == ffmpeg_next::util::format::Pixel::YUV420P {
            let width = frame_data.width as usize;
            let height = frame_data.height as usize;
            let y_size = width * height;
            let uv_size = y_size / 4;
            
            if frame_data.data.len() < y_size + 2 * uv_size {
                anyhow::bail!("Invalid YUV420P data size: expected {}, got {}", 
                             y_size + 2 * uv_size, frame_data.data.len());
            }
            
            // 设置Y平面
            let y_data = &frame_data.data[0..y_size];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    y_data.as_ptr(),
                    input_frame.data_mut(0).as_mut_ptr(),
                    y_size.min(input_frame.data(0).len())
                );
            }
            
            // 设置U平面
            let u_data = &frame_data.data[y_size..y_size + uv_size];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    u_data.as_ptr(),
                    input_frame.data_mut(1).as_mut_ptr(),
                    uv_size.min(input_frame.data(1).len())
                );
            }
            
            // 设置V平面
            let v_data = &frame_data.data[y_size + uv_size..y_size + 2 * uv_size];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    v_data.as_ptr(),
                    input_frame.data_mut(2).as_mut_ptr(),
                    uv_size.min(input_frame.data(2).len())
                );
            }
        } else {
            // 对于其他格式，使用原来的简单复制方法
            let data_len = frame_data.data.len();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    frame_data.data.as_ptr(),
                    input_frame.data_mut(0).as_mut_ptr(),
                    data_len.min(input_frame.data(0).len())
                );
            }
        }
        
        // 创建输出帧
        let mut output_frame = ffmpeg_next::util::frame::video::Video::empty();
        
        // 执行转换
        if let Some(ref mut scaler) = self.scaler {
            scaler.run(&input_frame, &mut output_frame)?;
        }
        
        // 提取RGB数据
        let rgb_size = (frame_data.width * frame_data.height * 3) as usize;
        let rgb_data = output_frame.data(0)[0..rgb_size].to_vec();
        
        Ok(rgb_data)
    }

} 