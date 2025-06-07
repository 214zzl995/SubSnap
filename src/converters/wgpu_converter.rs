use anyhow::Result;
use crate::converter::{YuvToRgbConverter, FrameData, ConversionMode};

/// WGPU GPU加速转换器
/// 
/// 使用GPU计算着色器进行YUV到RGB转换
/// 适合批量处理大量帧数据
pub struct WgpuConverter {
    processor: Option<crate::wgpu_processor::WgpuImageProcessor>,
}

impl WgpuConverter {
    pub async fn new() -> Result<Self> {
        let processor = crate::wgpu_processor::WgpuImageProcessor::new().await.ok();
        Ok(Self { processor })
    }
}

#[async_trait::async_trait(?Send)]
impl YuvToRgbConverter for WgpuConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>> {
        if let Some(ref mut processor) = self.processor {
            processor.convert_yuv420p_to_rgb(&frame_data.data, frame_data.width, frame_data.height).await
        } else {
            anyhow::bail!("WGPU processor not initialized")
        }
    }

    fn get_mode(&self) -> ConversionMode {
        ConversionMode::Wgpu
    }
} 