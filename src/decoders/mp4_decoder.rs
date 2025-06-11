use anyhow::{anyhow, Result};
use openh264::decoder::{Decoder as H264Decoder, DecoderConfig, Flush};
use std::fs::File;
use std::io::BufReader;
use super::{Decoder, FrameData, FrameDataPool, ProcessingResult};

// 这个方案性能很差 不会用的 如果能处理掉 rgb还要转 yuv 然后推过去 和多线程 最后的结果应该 比opencv 差20% 左右吧

/// Network abstraction layer type for H264 packet we might find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalType {
    Unspecified = 0,
    Slice = 1,
    Dpa = 2,
    Dpb = 3,
    Dpc = 4,
    IdrSlice = 5,
    Sei = 6,
    Sps = 7,
    Pps = 8,
    Aud = 9,
    EndSequence = 10,
    EndStream = 11,
    FillerData = 12,
    SpsExt = 13,
    Prefix = 14,
    SubSps = 15,
    Dps = 16,
    Reserved17 = 17,
    Reserved18 = 18,
    AuxiliarySlice = 19,
    ExtenSlice = 20,
    DepthExtenSlice = 21,
    Reserved22 = 22,
    Reserved23 = 23,
    Unspecified24 = 24,
    Unspecified25 = 25,
    Unspecified26 = 26,
    Unspecified27 = 27,
    Unspecified28 = 28,
    Unspecified29 = 29,
    Unspecified30 = 30,
    Unspecified31 = 31,
}

impl From<u8> for NalType {
    /// Reads NAL from header byte.
    fn from(value: u8) -> Self {
        use NalType::*;
        match value {
            0 => Unspecified,
            1 => Slice,
            2 => Dpa,
            3 => Dpb,
            4 => Dpc,
            5 => IdrSlice,
            6 => Sei,
            7 => Sps,
            8 => Pps,
            9 => Aud,
            10 => EndSequence,
            11 => EndStream,
            12 => FillerData,
            13 => SpsExt,
            14 => Prefix,
            15 => SubSps,
            16 => Dps,
            17 => Reserved17,
            18 => Reserved18,
            19 => AuxiliarySlice,
            20 => ExtenSlice,
            21 => DepthExtenSlice,
            22 => Reserved22,
            23 => Reserved23,
            24 => Unspecified24,
            25 => Unspecified25,
            26 => Unspecified26,
            27 => Unspecified27,
            28 => Unspecified28,
            29 => Unspecified29,
            30 => Unspecified30,
            31 => Unspecified31,
            _ => panic!("Invalid NAL type"),
        }
    }
}

/// A NAL unit in a bitstream.
struct NalUnit<'a> {
    nal_type: NalType,
    bytes: &'a [u8],
}

impl<'a> NalUnit<'a> {
    /// Reads a NAL unit from a slice of bytes in MP4, returning the unit, and the remaining stream after that slice.
    fn from_stream(mut stream: &'a [u8], length_size: u8) -> Option<(Self, &'a [u8])> {
        let mut nal_size = 0;

        // Construct nal_size from first bytes in MP4 stream.
        for _ in 0..length_size {
            nal_size = (nal_size << 8) | u32::from(stream[0]);
            stream = &stream[1..];
        }

        if nal_size == 0 {
            return None;
        }

        let packet = &stream[..nal_size as usize];
        let nal_type = NalType::from(packet[0] & 0x1F);
        let unit = NalUnit { nal_type, bytes: packet };

        stream = &stream[nal_size as usize..];

        Some((unit, stream))
    }
}

/// Converter from NAL units from the MP4 to the Annex B format expected by openh264.
///
/// It also inserts SPS and PPS units from the MP4 header into the stream.
/// They are also required for Annex B format to be decodable, but are not present in the MP4 bitstream,
/// as they are stored in the headers.
pub struct Mp4BitstreamConverter {
    length_size: u8,
    sps: Vec<Vec<u8>>,
    pps: Vec<Vec<u8>>,
    new_idr: bool,
    sps_seen: bool,
    pps_seen: bool,
}

impl Mp4BitstreamConverter {
    /// Create a new converter for the given track.
    ///
    /// The track must contain an AVC1 configuration.
    pub fn for_mp4_track(track: &mp4::Mp4Track) -> Result<Self> {
        let avcc_config = &track
            .trak
            .mdia
            .minf
            .stbl
            .stsd
            .avc1
            .as_ref()
            .ok_or_else(|| anyhow!("Track does not contain AVC1 config"))?
            .avcc;

        Ok(Self {
            length_size: avcc_config.length_size_minus_one + 1,
            sps: avcc_config.sequence_parameter_sets.iter().cloned().map(|v| v.bytes).collect(),
            pps: avcc_config.picture_parameter_sets.iter().cloned().map(|v| v.bytes).collect(),
            new_idr: true,
            sps_seen: false,
            pps_seen: false,
        })
    }

    /// Convert a single packet from the MP4 format to the Annex B format.
    ///
    /// It clears the `out` vector and appends the converted packet to it.
    pub fn convert_packet(&mut self, packet: &[u8], out: &mut Vec<u8>) {
        let mut stream = packet;
        out.clear();

        while !stream.is_empty() {
            let Some((unit, remaining_stream)) = NalUnit::from_stream(stream, self.length_size) else {
                continue;
            };

            stream = remaining_stream;

            match unit.nal_type {
                NalType::Sps => self.sps_seen = true,
                NalType::Pps => self.pps_seen = true,
                NalType::IdrSlice => {
                    // If this is a new IDR picture following an IDR picture, reset the idr flag.
                    // Just check first_mb_in_slice to be 1
                    if !self.new_idr && unit.bytes[1] & 0x80 != 0 {
                        self.new_idr = true;
                    }
                    // insert SPS & PPS NAL units if they were not seen
                    if self.new_idr && !self.sps_seen && !self.pps_seen {
                        self.new_idr = false;
                        for sps in &self.sps {
                            out.extend([0, 0, 1]);
                            out.extend(sps);
                        }
                        for pps in &self.pps {
                            out.extend([0, 0, 1]);
                            out.extend(pps);
                        }
                    }
                    // insert only PPS if SPS was seen
                    if self.new_idr && self.sps_seen && !self.pps_seen {
                        for pps in &self.pps {
                            out.extend([0, 0, 1]);
                            out.extend(pps);
                        }
                    }
                }
                _ => {}
            }

            out.extend([0, 0, 1]);
            out.extend(unit.bytes);

            if !self.new_idr && unit.nal_type == NalType::Slice {
                self.new_idr = true;
                self.sps_seen = false;
                self.pps_seen = false;
            }
        }
    }
}

pub struct Mp4Decoder {
    pool: FrameDataPool,
}

impl Mp4Decoder {
    pub fn new() -> Self {
        let estimated_frame_size = (3840 * 2160 * 3 / 2) as usize; // 假设最大4K分辨率
        Self {
            pool: FrameDataPool::new(16, estimated_frame_size),
        }
    }

    fn convert_rgb_to_yuv(&mut self, rgb_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let width = width as usize;
        let height = height as usize;
        let y_size = width * height;
        let uv_size = y_size / 4;
        let total_size = y_size + 2 * uv_size;
        
        // 从内存池获取预分配的缓冲区
        let mut yuv_data = self.pool.get_buffer(total_size);
        yuv_data.clear();
        yuv_data.reserve_exact(total_size);
        
        // 简单的RGB到YUV420P转换（基于ITU-R BT.601标准）
        for y in 0..height {
            for x in 0..width {
                let rgb_idx = (y * width + x) * 3;
                let r = rgb_data[rgb_idx] as f32;
                let g = rgb_data[rgb_idx + 1] as f32;
                let b = rgb_data[rgb_idx + 2] as f32;
                
                // Y分量
                let y_val = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                yuv_data.push(y_val);
            }
        }
        
        // U和V分量（2x2采样）
        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let rgb_idx = (y * width + x) * 3;
                let r = rgb_data[rgb_idx] as f32;
                let g = rgb_data[rgb_idx + 1] as f32;
                let b = rgb_data[rgb_idx + 2] as f32;
                
                // U分量 (Cb)
                let u_val = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) as u8;
                yuv_data.push(u_val);
            }
        }
        
        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let rgb_idx = (y * width + x) * 3;
                let r = rgb_data[rgb_idx] as f32;
                let g = rgb_data[rgb_idx + 1] as f32;
                let b = rgb_data[rgb_idx + 2] as f32;
                
                // V分量 (Cr)
                let v_val = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b) as u8;
                yuv_data.push(v_val);
            }
        }
        
        Ok(yuv_data)
    }
}

impl Decoder for Mp4Decoder {
    fn extract_frames_streaming(
        &mut self,
        input_path: &str,
        max_frames: u32,
        sample_fps: u32,
    ) -> Result<(ProcessingResult, Vec<FrameData>)> {
        let f = File::open(input_path)?;
        let size = f.metadata()?.len();
        let reader = BufReader::new(f);

        let mut mp4 = mp4::Mp4Reader::read_header(reader, size)?;
        let tracks = mp4.tracks();

        let (track_id, track) = tracks
            .into_iter()
            .find(|(_, t)| t.media_type().unwrap() == mp4::MediaType::H264)
            .ok_or_else(|| anyhow!("未找到H264视频轨道"))?;

        let width = track.width() as u32;
        let height = track.height() as u32;
        let track_id_owned = *track_id;
        let sample_count = mp4.sample_count(track_id_owned)?;

        // 计算视频时长和帧率
        let duration = track.duration().as_secs_f64();
        let video_duration_seconds = duration;
        let estimated_fps = sample_count as f64 / video_duration_seconds;
        
        let final_output_frames = if max_frames == 0 {
            if sample_fps > 0 {
                (video_duration_seconds * sample_fps as f64) as u32
            } else {
                sample_count
            }
        } else {
            max_frames.min(sample_count)
        };
        
        let frame_interval = if sample_fps > 0 {
            estimated_fps / sample_fps as f64
        } else if max_frames > 0 {
            sample_count as f64 / max_frames as f64
        } else {
            1.0
        };
        
        println!("MP4视频信息: 时长={:.2}秒, 总帧数={}, 目标输出帧数={}, 帧间隔={:.4}", 
                 video_duration_seconds, sample_count, final_output_frames, frame_interval);

        let mut bitstream_converter = Mp4BitstreamConverter::for_mp4_track(track)?;

        let decoder_options = unsafe {
            DecoderConfig::new()
                .flush_after_decode(Flush::NoFlush)
                .num_threads(0) // 使用自动线程数
        };

        let mut decoder = H264Decoder::with_api_config(
            openh264::OpenH264API::from_source(), 
            decoder_options
        )?;

        let mut buffer = Vec::new();
        let mut result_frames = Vec::new();
        let mut frame_count = 0;
        let mut next_sample_index = 1.0;
        let start_time = std::time::Instant::now();

        for i in 1..=sample_count {
            if frame_count >= final_output_frames {
                break;
            }

            // 如果设置了采样间隔，跳过不需要的帧
            if frame_interval > 1.0 && (i as f64) < next_sample_index {
                continue;
            }

            let Some(sample) = mp4.read_sample(track_id_owned, i)? else {
                continue;
            };

            bitstream_converter.convert_packet(&sample.bytes, &mut buffer);

            match decoder.decode(&buffer) {
                Ok(Some(image)) => {
                    frame_count += 1;
                    
                    // 使用write_rgb8方法提取RGB数据
                    let rgb_len = (width * height * 3) as usize;
                    let mut rgb_data = vec![0u8; rgb_len];
                    image.write_rgb8(&mut rgb_data);
                    
                    // 将RGB转换为YUV
                    let yuv_data = self.convert_rgb_to_yuv(&rgb_data, width, height)?;

                    let frame = FrameData {
                        frame_number: frame_count,
                        width,
                        height,
                        yuv_data,
                        format: ffmpeg_next::util::format::Pixel::YUV420P, // OpenH264总是输出YUV420P
                    };

                    result_frames.push(frame);
                    
                    if frame_interval > 1.0 {
                        next_sample_index += frame_interval;
                    }
                }
                Ok(None) => {
                    // 解码器还没准备好提供图像
                }
                Err(err) => {
                    println!("解码帧错误: {}", err);
                }
            }
        }

        // 处理剩余的帧
        for image in decoder.flush_remaining()? {
            if frame_count >= final_output_frames {
                break;
            }
            
            frame_count += 1;
            
            // 使用write_rgb8方法提取RGB数据
            let rgb_len = (width * height * 3) as usize;
            let mut rgb_data = vec![0u8; rgb_len];
            image.write_rgb8(&mut rgb_data);
            
            // 将RGB转换为YUV
            let yuv_data = self.convert_rgb_to_yuv(&rgb_data, width, height)?;

            let frame = FrameData {
                frame_number: frame_count,
                width,
                height,
                yuv_data,
                format: ffmpeg_next::util::format::Pixel::YUV420P,
            };

            result_frames.push(frame);
        }

        let total_duration = start_time.elapsed();

        Ok((ProcessingResult {
            frames_processed: frame_count,
            total_duration,
        }, result_frames))
    }
}
