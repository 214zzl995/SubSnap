use anyhow::Result;
use ffmpeg_next as ffmpeg;
use std::path::Path;

#[derive(Debug)]
pub struct ProcessingResult {
    pub frames_processed: u32,
    pub total_duration: std::time::Duration,
}

// 内存池结构，避免频繁分配
struct FrameDataPool {
    buffers: Vec<Vec<u8>>,
    current_index: usize,
}

impl FrameDataPool {
    fn new(capacity: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffers.push(Vec::with_capacity(buffer_size));
        }
        Self {
            buffers,
            current_index: 0,
        }
    }
    
    fn get_buffer(&mut self, required_size: usize) -> Vec<u8> {
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

pub async fn extract_frames_streaming(
    input_path: &str,
    max_frames: u32,
    sample_fps: u32,
    sender: tokio::sync::mpsc::Sender<crate::converters::ChannelFrameData>,
) -> Result<ProcessingResult> {
    use crate::converters::ChannelFrameData;

    let mut input = create_optimized_input_context(input_path)?;
    
    let video_stream_index = input.streams()
        .enumerate()
        .find(|(_, stream)| stream.parameters().medium() == ffmpeg::media::Type::Video)
        .map(|(i, _)| i)
        .ok_or_else(|| anyhow::anyhow!("未找到视频流"))?;

    let stream = input.streams().nth(video_stream_index).unwrap();
    let mut decoder_context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
    
    optimize_decoder_for_speed(&mut decoder_context)?;
    let mut decoder = decoder_context.decoder().video()?;

    let duration = stream.duration();
    let frame_rate = stream.avg_frame_rate();
    
    let video_duration_seconds = if duration > 0 {
        duration as f64 * f64::from(stream.time_base())
    } else {
        return Err(anyhow::anyhow!("无法获取视频时长信息"));
    };
    
    let total_video_frames = if frame_rate.numerator() > 0 && frame_rate.denominator() > 0 {
        let fps = frame_rate.numerator() as f64 / frame_rate.denominator() as f64;
        (video_duration_seconds * fps) as u32
    } else {
        return Err(anyhow::anyhow!("无法获取视频帧率信息"));
    };
    
    let final_output_frames = if max_frames == 0 {
        if sample_fps > 0 {
            (video_duration_seconds * sample_fps as f64) as u32
        } else {
            total_video_frames 
        }
    } else {
        max_frames
    };
    
    let frame_interval = if sample_fps > 0 {
        1.0 / sample_fps as f64
    } else if max_frames > 0 {
        video_duration_seconds / max_frames as f64
    } else {
        0.0 
    };
    
    println!("视频信息: 时长={:.2}秒, 总帧数={}, 目标输出帧数={}, 帧间隔={:.4}秒", 
             video_duration_seconds, total_video_frames, final_output_frames, frame_interval);
    
    // 初始化内存池，估算帧大小
    let estimated_frame_size = (3840 * 2160 * 3 / 2) as usize; // 假设最大4K分辨率
    let mut pool = FrameDataPool::new(16, estimated_frame_size); // 增大内存池容量
    
    let mut next_extract_time = 0.0;
    let mut frame_count = 0;
    let start_time = std::time::Instant::now();

    // 保持原始逻辑，只使用内存池优化
    for (stream, packet) in input.packets() {
        if stream.index() == video_stream_index && frame_count < final_output_frames {
            decoder.send_packet(&packet)?;

            let mut decoded = ffmpeg::util::frame::video::Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() && frame_count < final_output_frames {
                let timestamp = decoded.timestamp().unwrap_or(0) as f64 * f64::from(stream.time_base());
                
                let should_extract = if frame_interval == 0.0 {
                    true
                } else {
                    timestamp >= next_extract_time
                };
                
                if !should_extract {
                    continue;
                }
                
                if frame_interval > 0.0 {
                    next_extract_time += frame_interval;
                }
                
                frame_count += 1;
                
                let frame_data = extract_yuv_data_optimized(&decoded, &mut pool)?;

                let channel_frame = ChannelFrameData {
                    frame_number: frame_count,
                    width: decoded.width(),
                    height: decoded.height(),
                    yuv_data: frame_data,
                    format: decoded.format(),
                };

                if sender.send(channel_frame).await.is_err() {
                    break; 
                }
                
                if frame_count >= final_output_frames {
                    break;
                }
            }
        }
        
        if frame_count >= final_output_frames {
            break;
        }
    }

    let total_duration = start_time.elapsed();

    Ok(ProcessingResult {
        frames_processed: frame_count,
        total_duration,
    })
}

fn extract_yuv_data_optimized(decoded: &ffmpeg::util::frame::video::Video, pool: &mut FrameDataPool) -> Result<Vec<u8>> {
    if decoded.format() == ffmpeg::util::format::Pixel::YUV420P {
        let width = decoded.width() as usize;
        let height = decoded.height() as usize;
        let y_size = width * height;
        let uv_size = y_size / 4;
        let total_size = y_size + 2 * uv_size;
        
        // 从内存池获取预分配的缓冲区
        let mut frame_data = pool.get_buffer(total_size);
        frame_data.reserve_exact(total_size);
        
        // 获取各平面数据
        let y_plane = decoded.data(0);
        let y_stride = decoded.stride(0) as usize;
        let u_plane = decoded.data(1);
        let u_stride = decoded.stride(1) as usize;
        let v_plane = decoded.data(2);
        let v_stride = decoded.stride(2) as usize;
        
        let uv_width = width / 2;
        let uv_height = height / 2;
        
        // 高效拷贝Y平面
        if y_stride == width {
            // 无padding，一次性拷贝
            unsafe {
                let src_ptr = y_plane.as_ptr();
                let old_len = frame_data.len();
                frame_data.set_len(old_len + y_size);
                std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), y_size);
            }
        } else {
            // 有padding，批量逐行拷贝
            for y in 0..height {
                let src_offset = y * y_stride;
                unsafe {
                    let src_ptr = y_plane.as_ptr().add(src_offset);
                    let old_len = frame_data.len();
                    frame_data.set_len(old_len + width);
                    std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), width);
                }
            }
        }
        
        // 高效拷贝U平面
        if u_stride == uv_width {
            unsafe {
                let src_ptr = u_plane.as_ptr();
                let old_len = frame_data.len();
                frame_data.set_len(old_len + uv_size);
                std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), uv_size);
            }
        } else {
            for y in 0..uv_height {
                let src_offset = y * u_stride;
                unsafe {
                    let src_ptr = u_plane.as_ptr().add(src_offset);
                    let old_len = frame_data.len();
                    frame_data.set_len(old_len + uv_width);
                    std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), uv_width);
                }
            }
        }
        
        // 高效拷贝V平面
        if v_stride == uv_width {
            unsafe {
                let src_ptr = v_plane.as_ptr();
                let old_len = frame_data.len();
                frame_data.set_len(old_len + uv_size);
                std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), uv_size);
            }
        } else {
            for y in 0..uv_height {
                let src_offset = y * v_stride;
                unsafe {
                    let src_ptr = v_plane.as_ptr().add(src_offset);
                    let old_len = frame_data.len();
                    frame_data.set_len(old_len + uv_width);
                    std::ptr::copy_nonoverlapping(src_ptr, frame_data.as_mut_ptr().add(old_len), uv_width);
                }
            }
        }
        
        Ok(frame_data)
    } else {
        // 非YUV420P格式使用快速拷贝
        let data_size = decoded.data(0).len();
        let mut frame_data = pool.get_buffer(data_size);
        unsafe {
            frame_data.set_len(data_size);
            std::ptr::copy_nonoverlapping(decoded.data(0).as_ptr(), frame_data.as_mut_ptr(), data_size);
        }
        Ok(frame_data)
    }
}



fn optimize_decoder_for_speed(decoder_context: &mut ffmpeg::codec::context::Context) -> Result<()> {
    // 使用多线程解码
    decoder_context.set_threading(ffmpeg::threading::Config {
        kind: ffmpeg::threading::Type::Frame,
        count: 0, // 自动检测CPU核心数
    });
    
    Ok(())
}

fn create_optimized_input_context(input_path: &str) -> Result<ffmpeg::format::context::Input> {
    use ffmpeg::format;
    
    // 创建输入格式选项
    let mut format_opts = ffmpeg::Dictionary::new();
    
    // 设置更大的缓冲区大小和读取优化
    format_opts.set("buffer_size", "8388608"); // 8MB buffer (更大)
    format_opts.set("max_delay", "0"); // 无延迟
    format_opts.set("fflags", "fastseek+genpts"); // 快速seek + 生成PTS
    format_opts.set("analyzeduration", "500000"); // 进一步减少分析时间
    format_opts.set("probesize", "1000000"); // 进一步减少探测大小
    format_opts.set("max_probe_packets", "50"); // 限制探测包数量
    
    // 使用优化的格式选项打开输入
    let input = format::input_with_dictionary(&Path::new(input_path), format_opts)?;
    Ok(input)
} 