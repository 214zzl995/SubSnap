// 高性能 YUV420P 到 RGB 转换着色器

struct Params {
    width: u32,
    height: u32,
    y_plane_size: u32,
    uv_plane_size: u32,
}

@group(0) @binding(0) var<storage, read> y_plane: array<u32>;
@group(0) @binding(1) var<storage, read> u_plane: array<u32>;
@group(0) @binding(2) var<storage, read> v_plane: array<u32>;
@group(0) @binding(3) var<storage, read_write> rgb_data: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.width || y >= params.height) {
        return;
    }
    
    let width = params.width;
    let half_width = width / 2u;
    
    let y_idx = y * width + x;
    let uv_x = x / 2u;
    let uv_y = y / 2u;
    let uv_idx = uv_y * half_width + uv_x;
    
    let y_val = extract_byte_fast(y_plane[y_idx / 4u], y_idx % 4u);
    let u_val = extract_byte_fast(u_plane[uv_idx / 4u], uv_idx % 4u);
    let v_val = extract_byte_fast(v_plane[uv_idx / 4u], uv_idx % 4u);
    
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
    
    rgb_data[y_idx] = pack_rgb_fast(u32(r), u32(g), u32(b));
}

fn extract_byte_fast(packed: u32, offset: u32) -> u32 {
    return (packed >> (offset << 3u)) & 0xFFu;
}

fn saturate_fast(value: f32) -> f32 {
    return clamp(value, 0.0, 255.0);
}

fn pack_rgb_fast(r: u32, g: u32, b: u32) -> u32 {
    return 0xFF000000u | (b << 16u) | (g << 8u) | r;
} 