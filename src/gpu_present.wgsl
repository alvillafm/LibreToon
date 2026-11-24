struct Parameters {
    size: vec2<u32>,
    linear_target: u32,
    unused: u32,
    corners: array<vec4<f32>, 4>,
};
@group(0) @binding(0) var<storage, read> pixels: array<u32>;
@group(0) @binding(1) var<uniform> params: Parameters;
struct VertexOut { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) index: u32) -> VertexOut {
    let indices = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let uvs = array<vec2<f32>, 4>(vec2(0.,0.),vec2(1.,0.),vec2(1.,1.),vec2(0.,1.));
    let corner = indices[index];
    var out: VertexOut;
    out.position = vec4(params.corners[corner].xy, 0., 1.);
    out.uv = uvs[corner];
    return out;
}
fn pixel(at: vec2<i32>) -> vec4<f32> {
    let p = vec2<u32>(clamp(at, vec2<i32>(0), vec2<i32>(params.size)-vec2<i32>(1)));
    return unpack4x8unorm(pixels[p.y*params.size.x+p.x]);
}
fn linear_rgb(rgb: vec3<f32>) -> vec3<f32> {
    return select(rgb/12.92, pow((rgb+vec3(0.055))/1.055,vec3(2.4)),rgb>vec3(0.04045));
}
@fragment fn fs(input: VertexOut) -> @location(0) vec4<f32> {
    let coordinate = input.uv*vec2<f32>(params.size)-vec2(0.5);
    let at = vec2<i32>(floor(coordinate));
    let fraction = fract(coordinate);
    let a = mix(pixel(at),pixel(at+vec2<i32>(1,0)),fraction.x);
    let b = mix(pixel(at+vec2<i32>(0,1)),pixel(at+vec2<i32>(1,1)),fraction.x);
    let color = mix(a,b,fraction.y);
    if params.linear_target != 0u {return vec4(linear_rgb(color.rgb),color.a);}
    return color;
}
