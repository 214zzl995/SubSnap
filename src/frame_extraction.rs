use anyhow::Result;
use ffmpeg_next as ffmpeg;
use std::path::Path;

#[derive(Debug)]
pub struct ProcessingResult {
    pub frames_processed: u32,
    pub total_duration: std::time::Duration,
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
    
    let mut next_extract_time = 0.0;
    let mut frame_count = 0;
    let start_time = std::time::Instant::now();

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
                
                let frame_data = extract_yuv_data(&decoded)?;

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

fn extract_yuv_data(decoded: &ffmpeg::util::frame::video::Video) -> Result<Vec<u8>> {
    let mut frame_data = Vec::new();
    
    if decoded.format() == ffmpeg::util::format::Pixel::YUV420P {
        let width = decoded.width() as usize;
        let height = decoded.height() as usize;
        
        let y_plane = decoded.data(0);
        let y_stride = decoded.stride(0) as usize;
        for y in 0..height {
            let start = y * y_stride;
            let end = start + width;
            frame_data.extend_from_slice(&y_plane[start..end]);
        }
        
        let u_plane = decoded.data(1);
        let u_stride = decoded.stride(1) as usize;
        let uv_width = width / 2;
        let uv_height = height / 2;
        for y in 0..uv_height {
            let start = y * u_stride;
            let end = start + uv_width;
            frame_data.extend_from_slice(&u_plane[start..end]);
        }
            
        let v_plane = decoded.data(2);
        let v_stride = decoded.stride(2) as usize;
        for y in 0..uv_height {
            let start = y * v_stride;
            let end = start + uv_width;
            frame_data.extend_from_slice(&v_plane[start..end]);
        }
    } else {
        let data_size = decoded.data(0).len();
        frame_data = vec![0u8; data_size];
        frame_data.copy_from_slice(decoded.data(0));
    }
    
    Ok(frame_data)
}

fn optimize_decoder_for_speed(decoder_context: &mut ffmpeg::codec::context::Context) -> Result<()> {
    decoder_context.set_threading(ffmpeg::threading::Config {
        kind: ffmpeg::threading::Type::Frame,
        count: 0,
    });
    Ok(())
}

fn create_optimized_input_context(input_path: &str) -> Result<ffmpeg::format::context::Input> {
    use ffmpeg::format;
    let input = format::input(&Path::new(input_path))?;
    Ok(input)
} 