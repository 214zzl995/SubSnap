use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum DecoderType {
    /// 使用FFmpeg解码器
    #[value(name = "ffmpeg")]
    FFmpeg,
    /// 使用OpenCV解码器
    #[value(name = "opencv")]
    OpenCV,
}

impl DecoderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DecoderType::FFmpeg => "ffmpeg",
            DecoderType::OpenCV => "opencv",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DecoderType::FFmpeg => "使用FFmpeg库进行视频解码",
            DecoderType::OpenCV => "使用OpenCV库进行视频解码",
        }
    }
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub frames_processed: u32,
    pub total_duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct FrameData {
    pub frame_number: u32,
    pub width: u32,
    pub height: u32,
    pub yuv_data: Vec<u8>,
    pub format: ffmpeg_next::util::format::Pixel,
}

impl From<FrameData> for crate::converters::ChannelFrameData {
    fn from(frame_data: FrameData) -> Self {
        Self {
            frame_number: frame_data.frame_number,
            width: frame_data.width,
            height: frame_data.height,
            yuv_data: frame_data.yuv_data,
            format: frame_data.format,
        }
    }
}

impl From<crate::converters::ChannelFrameData> for FrameData {
    fn from(channel_frame_data: crate::converters::ChannelFrameData) -> Self {
        Self {
            frame_number: channel_frame_data.frame_number,
            width: channel_frame_data.width,
            height: channel_frame_data.height,
            yuv_data: channel_frame_data.yuv_data,
            format: channel_frame_data.format,
        }
    }
}

// 内存池结构，避免频繁分配
#[derive(Debug)]
pub struct FrameDataPool {
    buffers: Vec<Vec<u8>>,
    current_index: usize,
}

impl FrameDataPool {
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffers.push(Vec::with_capacity(buffer_size));
        }
        Self {
            buffers,
            current_index: 0,
        }
    }
    
    pub fn get_buffer(&mut self, required_size: usize) -> Vec<u8> {
        let current_idx = self.current_index;
        self.current_index = (self.current_index + 1) % self.buffers.len();
        
        let mut buffer = std::mem::take(&mut self.buffers[current_idx]);
        buffer.clear();
        if buffer.capacity() < required_size {
            buffer.reserve(required_size - buffer.capacity());
        }
        buffer
    }
}

// 抽象解码器接口
pub trait Decoder: Send {
    fn extract_frames_streaming(
        &mut self,
        input_path: &str,
        max_frames: u32,
        sample_fps: u32,
    ) -> Result<(ProcessingResult, Vec<FrameData>)>;
}

pub struct DecoderFactory;

impl DecoderFactory {
    pub fn create_decoder(decoder_type: DecoderType) -> Result<Box<dyn Decoder>> {
        match decoder_type {
            DecoderType::FFmpeg => Ok(Box::new(ffmpeg_decoder::FFmpegDecoder::new())),
            DecoderType::OpenCV => Ok(Box::new(opencv_decoder::OpenCVDecoder::new())),
        }
    }

    pub fn available_decoders() -> Vec<DecoderType> {
        vec![
            DecoderType::FFmpeg,
            DecoderType::OpenCV,
        ]
    }
}



// 新的解码器使用函数
pub async fn extract_frames_with_decoder(
    decoder_type: DecoderType,
    input_path: &str,
    max_frames: u32,
    sample_fps: u32,
    sender: tokio::sync::mpsc::Sender<crate::converters::ChannelFrameData>,
) -> Result<ProcessingResult> {
    let input_path = input_path.to_string();
    let (result, frames) = tokio::task::spawn_blocking(move || {
        let mut decoder = DecoderFactory::create_decoder(decoder_type)?;
        decoder.extract_frames_streaming(&input_path, max_frames, sample_fps)
    }).await??;
    
    // 异步发送帧数据
    for frame in frames {
        let channel_frame: crate::converters::ChannelFrameData = frame.into();
        if sender.send(channel_frame).await.is_err() {
            break;
        }
    }
    
    Ok(result)
}

pub mod ffmpeg_decoder;
pub mod opencv_decoder; 