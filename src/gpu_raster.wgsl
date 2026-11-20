@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var<uniform> textured: vec4<u32>;
struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}
@vertex fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Vertex {
    var out: Vertex; out.position=vec4<f32>(position,0.0,1.0);out.uv=uv;out.color=color;return out;
}
fn premul(p: vec4<f32>) -> vec4<f32> {return vec4<f32>(p.xyz*p.w,p.w);}
@fragment fn fs(v: Vertex) -> @location(0) vec4<f32> {
    if textured.x==0u {return v.color;}
    let size=vec2<i32>(textureDimensions(image));
    let xy=clamp(v.uv*vec2<f32>(size)-vec2<f32>(0.5),vec2<f32>(0.0),vec2<f32>(size-vec2<i32>(1)));
    let p=vec2<i32>(xy);let q=min(p+vec2<i32>(1),size-vec2<i32>(1));let t=fract(xy);
    let a=premul(textureLoad(image,p,0));let b=premul(textureLoad(image,vec2<i32>(q.x,p.y),0));
    let c=premul(textureLoad(image,vec2<i32>(p.x,q.y),0));let d=premul(textureLoad(image,q,0));
    return ((a*(1.0-t.x)+b*t.x)*(1.0-t.y)+(c*(1.0-t.x)+d*t.x)*t.y)*v.color.w;
}
