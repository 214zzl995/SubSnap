use anyhow::Result;
use std::time::Instant;

// 导入转换器模块
use crate::converters::*;

// 统一的转换模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConversionMode {
    Ffmpeg,   // 使用FFmpeg SWScale
    Wgpu,     // 使用WGPU GPU加速
    Yuvutils, // 使用yuvutils-rs
    Opencv,   // 使用OpenCV库
    Manual,   // 手工实现
}

impl ConversionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversionMode::Ffmpeg => "ffmpeg",
            ConversionMode::Wgpu => "wgpu", 
            ConversionMode::Yuvutils => "yuvutils",
            ConversionMode::Opencv => "opencv",
            ConversionMode::Manual => "manual",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ConversionMode::Ffmpeg => "使用FFmpeg SWScale进行CPU转换",
            ConversionMode::Wgpu => "使用WGPU进行GPU加速转换",
            ConversionMode::Yuvutils => "使用yuvutils-rs进行高性能CPU转换",
            ConversionMode::Opencv => "使用OpenCV库进行CPU转换",
            ConversionMode::Manual => "使用手工实现进行CPU转换",
        }
    }
}

// 统一的帧数据结构
#[derive(Clone, Debug)]
pub struct FrameData {
    pub frame_number: u32,
    pub timestamp: f64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ffmpeg_next::util::format::Pixel,
}

// 转换性能统计
#[derive(Debug, Default)]
pub struct ConversionStats {
    pub frames_processed: u32,
    pub total_time_ms: u64,
    pub avg_time_per_frame_ms: f64,
    pub fps: f64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
}

impl ConversionStats {
    pub fn new() -> Self {
        Self {
            min_time_ms: u64::MAX,
            max_time_ms: 0,
            ..Default::default()
        }
    }

    pub fn record_frame(&mut self, duration_ms: u64) {
        self.frames_processed += 1;
        self.total_time_ms += duration_ms;
        self.min_time_ms = self.min_time_ms.min(duration_ms);
        self.max_time_ms = self.max_time_ms.max(duration_ms);
        self.avg_time_per_frame_ms = self.total_time_ms as f64 / self.frames_processed as f64;
        if self.avg_time_per_frame_ms > 0.0 {
            self.fps = 1000.0 / self.avg_time_per_frame_ms;
        }
    }

    pub fn print_summary(&self, mode: ConversionMode) {
        println!("\n📊 {} 转换性能统计:", mode.description());
        println!("  🎞️  处理帧数: {}", self.frames_processed);
        println!("  ⏱️  总耗时: {:.2}秒", self.total_time_ms as f64 / 1000.0);
        println!("  📈 平均每帧: {:.2}ms", self.avg_time_per_frame_ms);
        println!("  ⚡ 最快耗时: {}ms", self.min_time_ms);
        println!("  🐌 最慢耗时: {}ms", self.max_time_ms);
        println!("  🚀 转换FPS: {:.1}", self.fps);
    }
}

// 统一的转换器trait
#[async_trait::async_trait(?Send)]
pub trait YuvToRgbConverter {
    async fn convert(&mut self, frame_data: &FrameData) -> Result<Vec<u8>>;
    #[allow(dead_code)]
    fn get_mode(&self) -> ConversionMode;
    async fn cleanup(&mut self) -> Result<()> { Ok(()) }
}

// 转换器工厂
pub struct ConverterFactory;

impl ConverterFactory {
    pub async fn create_converter(mode: ConversionMode) -> Result<Box<dyn YuvToRgbConverter>> {
        match mode {
            ConversionMode::Ffmpeg => {
                Ok(Box::new(FfmpegConverter::new()))
            }
            #[cfg(feature = "wgpu-mode")]
            ConversionMode::Wgpu => {
                Ok(Box::new(WgpuConverter::new().await?))
            }
            #[cfg(not(feature = "wgpu-mode"))]
            ConversionMode::Wgpu => {
                anyhow::bail!("WGPU mode not enabled. Please compile with --features wgpu-mode")
            }
            #[cfg(feature = "yuvutils-mode")]
            ConversionMode::Yuvutils => {
                Ok(Box::new(YuvutilsConverter::new()))
            }
            #[cfg(not(feature = "yuvutils-mode"))]
            ConversionMode::Yuvutils => {
                anyhow::bail!("Yuvutils mode not enabled. Please compile with --features yuvutils-mode")
            }
            #[cfg(feature = "opencv-mode")]
            ConversionMode::Opencv => {
                Ok(Box::new(OpencvConverter::new()))
            }
            #[cfg(not(feature = "opencv-mode"))]
            ConversionMode::Opencv => {
                anyhow::bail!("OpenCV mode not enabled. Please compile with --features opencv-mode")
            }
            ConversionMode::Manual => {
                Ok(Box::new(ManualConverter::new()))
            }
        }
    }

    pub fn available_modes() -> Vec<ConversionMode> {
        let mut modes = vec![ConversionMode::Ffmpeg, ConversionMode::Manual];
        
        #[cfg(feature = "opencv-mode")]
        modes.push(ConversionMode::Opencv);
        
        #[cfg(feature = "wgpu-mode")]
        modes.push(ConversionMode::Wgpu);
        
        #[cfg(feature = "yuvutils-mode")]
        modes.push(ConversionMode::Yuvutils);
        
        modes
    }
}

// 性能基准测试
pub struct Benchmark {
    pub stats: std::collections::HashMap<ConversionMode, ConversionStats>,
}

impl Benchmark {
    pub fn new() -> Self {
        Self {
            stats: std::collections::HashMap::new(),
        }
    }

    pub async fn run_conversion_test(
        &mut self,
        mode: ConversionMode,
        frames: &[FrameData],
    ) -> Result<()> {
        println!("🚀 开始测试 {} 模式...", mode.description());
        
        let mut converter = ConverterFactory::create_converter(mode).await?;
        let mut stats = ConversionStats::new();
        
        for frame in frames {
            let start = Instant::now();
            
            match converter.convert(frame).await {
                Ok(rgb_data) => {
                    let duration = start.elapsed();
                    // 使用微秒精度，然后转换为毫秒
                    let duration_us = duration.as_micros() as u64;
                    let duration_ms = std::cmp::max(1, duration_us / 1000); // 最少1ms
                    stats.record_frame(duration_ms);
                    
                    if stats.frames_processed <= 3 {
                        if duration_us < 1000 {
                            println!(
                                "  ✅ 帧#{}: {}x{} -> RGB ({} bytes) 耗时: {}μs",
                                frame.frame_number,
                                frame.width,
                                frame.height,
                                rgb_data.len(),
                                duration_us
                            );
                        } else {
                            println!(
                                "  ✅ 帧#{}: {}x{} -> RGB ({} bytes) 耗时: {}ms",
                                frame.frame_number,
                                frame.width,
                                frame.height,
                                rgb_data.len(),
                                duration_ms
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ 帧#{} 转换失败: {}", frame.frame_number, e);
                }
            }
        }
        
        converter.cleanup().await?;
        
        stats.print_summary(mode);
        self.stats.insert(mode, stats);
        
        Ok(())
    }

    pub fn print_comparison(&self) {
        if self.stats.len() < 2 {
            return;
        }

        println!("\n🏆 性能对比总结:");
        println!("┌─────────────────┬──────────┬──────────┬──────────┬──────────┬──────────┐");
        println!("│      模式       │ 帧数     │ 平均耗时 │   FPS    │ 最快耗时 │ 最慢耗时 │");
        println!("├─────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┤");
        
        let mut modes: Vec<_> = self.stats.keys().collect();
        modes.sort_by_key(|&mode| (*mode as u8));
        
        for &mode in &modes {
            let stats = &self.stats[mode];
            println!(
                "│ {:15} │ {:8} │ {:6.2}ms │ {:6.1}   │ {:6}ms │ {:6}ms │",
                mode.as_str(),
                stats.frames_processed,
                stats.avg_time_per_frame_ms,
                stats.fps,
                stats.min_time_ms,
                stats.max_time_ms
            );
        }
        
        println!("└─────────────────┴──────────┴──────────┴──────────┴──────────┴──────────┘");

        // 找出最快的模式
        if let Some((&fastest_mode, fastest_stats)) = self.stats.iter()
            .max_by(|a, b| a.1.fps.partial_cmp(&b.1.fps).unwrap_or(std::cmp::Ordering::Equal)) {
            println!("🥇 最快模式: {} ({:.1} FPS)", fastest_mode.as_str(), fastest_stats.fps);
        }

        // 性能分析和建议
        self.print_performance_analysis();
    }

    fn print_performance_analysis(&self) {
        println!("\n📈 性能分析和建议:");
        
        for (&mode, stats) in &self.stats {
            let variance = if stats.max_time_ms > stats.min_time_ms {
                stats.max_time_ms - stats.min_time_ms
            } else { 0 };
            
            let stability = if variance <= 5 { "稳定" } 
                          else if variance <= 20 { "一般" } 
                          else { "不稳定" };
                          
            println!("  {} - {:.1} FPS (性能稳定性: {})", 
                     mode.description(), stats.fps, stability);
                     
            match mode {
                ConversionMode::Ffmpeg => {
                    if stats.fps > 500.0 {
                        println!("    ✅ 推荐用于生产环境的高性能需求");
                    }
                }
                ConversionMode::Yuvutils => {
                    if stats.fps > 50.0 {
                        println!("    ✅ 推荐用于纯Rust环境的高性能需求");
                    }
                }
                ConversionMode::Wgpu => {
                    if stats.fps < 30.0 {
                        println!("    ⚠️  GPU模式适合大批量处理，小批量可能不如CPU模式");
                    }
                }
                ConversionMode::Opencv => {
                    println!("    📚 适合使用OpenCV库进行标准化转换");
                }
                ConversionMode::Manual => {
                    println!("    📚 适合学习和理解YUV转换原理");
                }
            }
        }
    }
} 