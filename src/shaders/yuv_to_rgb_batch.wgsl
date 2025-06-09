// 批处理YUV420P到RGB转换着色器 - 优化的RGB输出

struct BatchParams {
    width: u32,
    height: u32,
    y_plane_size: u32,
    uv_plane_size: u32,
}

@group(0) @binding(0) var<storage, read> y_plane: array<u32>;
@group(0) @binding(1) var<storage, read> u_plane: array<u32>;
@group(0) @binding(2) var<storage, read> v_plane: array<u32>;
@group(0) @binding(3) var<storage, read_write> rgb_data: array<u32>;  // RGB数据，每个u32包含一个像素的RGB+填充
@group(0) @binding(4) var<uniform> params: BatchParams;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let frame_idx = global_id.z; // 批处理中的帧索引
    
    let width = params.width;
    let height = params.height;
    
    if (x >= width || y >= height) {
        return;
    }
    
    let half_width = width / 2u;
    let frame_y_size = width * height;
    let frame_uv_size = frame_y_size / 4u;
    
    // 计算当前帧在批处理数据中的偏移
    let frame_y_offset = frame_idx * frame_y_size;
    let frame_u_offset = frame_idx * frame_uv_size;
    let frame_v_offset = frame_idx * frame_uv_size;
    let frame_output_offset = frame_idx * frame_y_size; // RGB输出偏移（每像素一个u32）
    
    // 当前像素在帧内的位置
    let y_idx = y * width + x;
    let uv_x = x / 2u;
    let uv_y = y / 2u;
    let uv_idx = uv_y * half_width + uv_x;
    
    // 在批处理数据中的绝对位置
    let batch_y_idx = frame_y_offset + y_idx;
    let batch_u_idx = frame_u_offset + uv_idx;
    let batch_v_idx = frame_v_offset + uv_idx;
    let batch_output_idx = frame_output_offset + y_idx;
    
    // 提取YUV值
    let y_val = extract_byte_fast(y_plane[batch_y_idx / 4u], batch_y_idx % 4u);
    let u_val = extract_byte_fast(u_plane[batch_u_idx / 4u], batch_u_idx % 4u);
    let v_val = extract_byte_fast(v_plane[batch_v_idx / 4u], batch_v_idx % 4u);
    
    // YUV到RGB转换
    let y_f = f32(y_val);
    let u_f = f32(u_val) - 128.0;
    let v_f = f32(v_val) - 128.0;
    
    let y_contrib = (y_f - 16.0) * 1.164;
    var r = y_contrib + v_f * 1.793;
    var g = y_contrib - u_f * 0.213 - v_f * 0.533;
    var b = y_contrib + u_f * 2.112;
    
    r = saturate_fast(r);
    g = saturate_fast(g);
    b = saturate_fast(b);
    
    // 输出RGB格式（紧凑排列：R|G|B|0）
    rgb_data[batch_output_idx] = pack_rgb_compact(u32(r), u32(g), u32(b));
}

fn extract_byte_fast(packed: u32, offset: u32) -> u32 {
    return (packed >> (offset << 3u)) & 0xFFu;
}

fn saturate_fast(value: f32) -> f32 {
    return clamp(value, 0.0, 255.0);
}

// 紧凑的RGB打包：R|G|B|0
fn pack_rgb_compact(r: u32, g: u32, b: u32) -> u32 {
    return r | (g << 8u) | (b << 16u);
} 