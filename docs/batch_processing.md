# WGPU批处理转换功能

## 概述

SubSnap现在支持WGPU批处理转换，可以显著提高YUV到RGB转换的性能。批处理模式通过一次性处理多个帧来减少GPU调用开销和内存传输开销。

## 功能特性

### 1. 全局批处理池
- 单例模式的全局池，可在整个应用程序中共享
- 自动收集帧并在达到批次大小或超时时进行处理
- 默认批次大小：8帧
- 默认超时：100毫秒

### 2. 本地批处理池
- 实例化的本地池，可配置独立的批次参数
- 支持自定义批次大小和超时时间
- 适合特定场景的优化需求

### 3. 兼容性
- 完全兼容现有的`YuvToRgbConverter`接口
- 可无缝替换现有的单帧转换器

## 使用方法

### 方法1：通过CLI使用批处理模式

```bash
# 使用WGPU批处理模式
cargo run -- --mode wgpu-batch --input video.mp4 --frames 100

# 对比性能（先运行单帧模式）
cargo run -- --mode wgpu --input video.mp4 --frames 100
```

### 方法2：编程接口使用

#### 全局池批处理
```rust
use sub_snap::converters::wgpu_converter::get_global_batch_pool;

async fn process_frames_with_global_pool() -> Result<()> {
    let pool = get_global_batch_pool().await;
    let mut all_results = Vec::new();
    
    for frame in frames {
        let mut pool_guard = pool.lock().await;
        if let Some(results) = pool_guard.add_frame(&frame).await? {
            all_results.extend(results);
        }
    }
    
    // 处理剩余的帧
    let mut pool_guard = pool.lock().await;
    if let Some(results) = pool_guard.flush().await? {
        all_results.extend(results);
    }
    
    Ok(())
}
```

#### 本地池批处理
```rust
use sub_snap::converters::wgpu_converter::WgpuBatchConverter;

async fn process_frames_with_local_pool() -> Result<()> {
    // 创建本地批处理转换器
    // 参数：use_global_pool=false, batch_size=16, max_wait_time_ms=50
    let mut converter = WgpuBatchConverter::new(false, Some(16), Some(50)).await?;
    let mut all_results = Vec::new();
    
    for frame in frames {
        if let Some(results) = converter.add_frame(&frame).await? {
            all_results.extend(results);
        }
    }
    
    // 处理剩余的帧
    if let Some(results) = converter.flush().await? {
        all_results.extend(results);
    }
    
    Ok(())
}
```

#### 作为标准转换器使用
```rust
use sub_snap::converters::{YuvToRgbConverter, ConverterFactory, ConversionMode};

async fn use_as_standard_converter() -> Result<()> {
    // 通过工厂创建批处理转换器
    let mut converter = ConverterFactory::create_converter(ConversionMode::WGPUBatch).await?;
    
    // 像普通转换器一样使用
    for frame in frames {
        let rgb_data = converter.convert(&frame).await?;
        // 处理RGB数据...
    }
    
    // 清理资源
    converter.cleanup().await?;
    Ok(())
}
```

## 性能优势

### 批处理的优势
1. **减少GPU调用开销**：批量提交计算任务而不是逐帧提交
2. **优化内存传输**：一次性传输多帧数据到GPU
3. **提高GPU利用率**：更好地利用GPU的并行计算能力
4. **减少CPU-GPU同步**：减少等待GPU完成的次数

### 适用场景
- **高帧率视频处理**：大量帧需要快速转换
- **批量图像处理**：处理大量静态图像
- **实时视频流**：可配置小批次和短超时实现低延迟
- **离线视频处理**：可配置大批次实现最高吞吐量

### 性能调优建议

#### 批次大小选择
- **小批次（2-4帧）**：适合实时应用，延迟敏感
- **中批次（8-16帧）**：平衡延迟和吞吐量，适合大多数场景
- **大批次（32-64帧）**：最大化吞吐量，适合离线处理

#### 超时时间设置
- **短超时（10-50ms）**：实时应用，保证响应性
- **中超时（100-200ms）**：一般应用，平衡性能和延迟
- **长超时（500ms+）**：离线处理，最大化批次利用率

## 配置参数

### 全局池配置
```rust
// 全局池使用固定配置：
// - 批次大小：8帧
// - 超时时间：100毫秒
let pool = get_global_batch_pool().await;
```

### 本地池配置
```rust
// 自定义配置示例
let converter = WgpuBatchConverter::new(
    false,          // 不使用全局池
    Some(16),       // 批次大小：16帧
    Some(50)        // 超时时间：50毫秒
).await?;
```

## 注意事项

### 限制条件
1. **相同尺寸要求**：批处理中的所有帧必须具有相同的宽度和高度
2. **内存使用**：批处理会增加内存使用，大批次需要更多GPU内存
3. **延迟权衡**：批处理可能增加单帧的处理延迟

### 错误处理
- 如果帧尺寸不一致，批处理会返回错误
- GPU内存不足时会自动回退到较小的批次
- 超时触发时会处理当前累积的所有帧

### 线程安全
- 全局池使用`Arc<Mutex<>>`确保线程安全
- 本地池不是线程安全的，每个线程应使用独立实例

## 示例场景

### 场景1：实时视频流处理
```rust
// 低延迟配置
let mut converter = WgpuBatchConverter::new(false, Some(4), Some(16)).await?;
```

### 场景2：批量视频转换
```rust
// 高吞吐量配置
let mut converter = WgpuBatchConverter::new(false, Some(32), Some(500)).await?;
```

### 场景3：混合工作负载
```rust
// 使用全局池，让系统自动平衡
let pool = get_global_batch_pool().await;
```

## 性能监控

建议在应用中添加性能监控：

```rust
use std::time::Instant;

let start = Instant::now();
let results = converter.add_frame(&frame).await?;
let duration = start.elapsed();

if let Some(batch_results) = results {
    println!("批处理了 {} 帧，耗时 {:?}", batch_results.len(), duration);
}
```

通过批处理功能，SubSnap在处理大量YUV帧时可以获得显著的性能提升，特别是在GPU性能充足的系统上。 