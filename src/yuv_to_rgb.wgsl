// 高性能 YUV420P 到 RGB 转换着色器

struct Params {
    width: u32,
    height: u32,
    y_plane_size: u32,
    uv_plane_size: u32,
}

// 使用字节数组而不是u32打包数据，减少位操作开销
@group(0) @binding(0) var<storage, read> y_plane: array<u32>;
@group(0) @binding(1) var<storage, read> u_plane: array<u32>;
@group(0) @binding(2) var<storage, read> v_plane: array<u32>;
@group(0) @binding(3) var<storage, read_write> rgb_data: array<u8>; // Changed to array<u8>
@group(0) @binding(4) var<uniform> params: Params;

// 优化工作组大小为16x16，更好的GPU占用率
@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.width || y >= params.height) {
        return;
    }
    
    // 使用常量避免重复计算
    let width = params.width;
    let half_width = width / 2u;
    
    // 优化内存访问模式
    let y_idx = y * width + x;
    let uv_x = x / 2u;
    let uv_y = y / 2u;
    let uv_idx = uv_y * half_width + uv_x;
    
    // 直接从分离的平面读取，减少偏移计算
    let y_val = extract_byte_fast(y_plane[y_idx / 4u], y_idx % 4u);
    let u_val = extract_byte_fast(u_plane[uv_idx / 4u], uv_idx % 4u);
    let v_val = extract_byte_fast(v_plane[uv_idx / 4u], uv_idx % 4u);
    
    // 使用预计算的常量加速转换
    let y_f = f32(y_val);
    let u_f = f32(u_val) - 128.0;
    let v_f = f32(v_val) - 128.0;
    
    // 优化的BT.709转换（合并常量）
    let y_contrib = (y_f - 16.0) * 1.164;
    var r = y_contrib + v_f * 1.793;
    var g = y_contrib - u_f * 0.213 - v_f * 0.533;
    var b = y_contrib + u_f * 2.112;
    
    // 快速饱和度限制
    r = saturate_fast(r);
    g = saturate_fast(g);
    b = saturate_fast(b);
    
    // Calculate base byte index for RGB output
    let base_idx = y_idx * 3u;

    // Write R, G, B components directly as bytes
    rgb_data[base_idx + 0u] = u8(r);
    rgb_data[base_idx + 1u] = u8(g);
    rgb_data[base_idx + 2u] = u8(b);
}

// 优化的字节提取函数
fn extract_byte_fast(packed: u32, offset: u32) -> u32 {
    return (packed >> (offset << 3u)) & 0xFFu;
}

// 快速饱和度限制
fn saturate_fast(value: f32) -> f32 {
    return clamp(value, 0.0, 255.0);
}

// pack_rgb_fast function is no longer needed


