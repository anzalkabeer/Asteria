struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Convert ASTERIA coordinate space (top-left 0,0, pixel coordinates) 
    // to WebGPU Clip Space (-1.0 to 1.0, Y-up)
    // For now we'll assume a fixed 800x600 viewport for this milestone
    let x = (model.position.x / 800.0) * 2.0 - 1.0;
    let y = 1.0 - (model.position.y / 600.0) * 2.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
