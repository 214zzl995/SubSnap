use anyhow::Result;
use super::{Decoder, FrameData, FrameDataPool, ProcessingResult};

pub struct OpenCVDecoder {
    _pool: FrameDataPool,
}

impl OpenCVDecoder {
    pub fn new() -> Self {
        let estimated_frame_size = (3840 * 2160 * 3 / 2) as usize; // 假设最大4K分辨率
        Self {
            _pool: FrameDataPool::new(16, estimated_frame_size),
        }
    }
}

impl Decoder for OpenCVDecoder {
    fn extract_frames_streaming(
        &mut self,
        input_path: &str,
        max_frames: u32,
        sample_fps: u32,
    ) -> Result<(ProcessingResult, Vec<FrameData>)> {
        // 暂时委托给 FFmpeg 解码器，这样可以保持接口但避免 OpenCV 复杂性
        // 在未来的版本中，可以实现真正的 OpenCV 解码逻辑
        println!("使用 OpenCV 解码器（当前委托给 FFmpeg 实现）");
        let mut ffmpeg_decoder = super::ffmpeg_decoder::FFmpegDecoder::new();
        ffmpeg_decoder.extract_frames_streaming(input_path, max_frames, sample_fps)
    }
} 