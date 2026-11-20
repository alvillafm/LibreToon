@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read> params:array<u32>;
@group(0) @binding(2) var<storage,read_write> output:array<u32>;
fn f(i:u32)->f32{return bitcast<f32>(params[i]);}
fn sample_at(p:vec2<i32>)->vec4<f32>{
    if any(p<vec2<i32>(0))||p.x>=i32(params[2])||p.y>=i32(params[3]){return vec4<f32>(0.0);}
    let value=source[u32(p.y)*params[2]+u32(p.x)];
    return vec4<f32>(f32(value&255u),f32((value>>8u)&255u),f32((value>>16u)&255u),f32(value>>24u));
}
@compute @workgroup_size(16,16)
fn main(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[0]||p.y>=params[1]{return;}
    let start=vec2<f32>(f(4u),f(5u))+vec2<f32>(f(8u),f(9u))*f32(p.y);
    let at=start+vec2<f32>(f(6u),f(7u))*f32(p.x);let i=vec2<i32>(floor(at));let t=at-vec2<f32>(i);
    let a=sample_at(i);let b=sample_at(i+vec2<i32>(1,0));let c=sample_at(i+vec2<i32>(0,1));let d=sample_at(i+vec2<i32>(1,1));
    let top=a*(1.0-t.x)+b*t.x;let bottom=c*(1.0-t.x)+d*t.x;
    let rgba=vec4<u32>(floor(top*(1.0-t.y)+bottom*t.y+vec4<f32>(0.5)));
    output[p.y*params[0]+p.x]=rgba.x|(rgba.y<<8u)|(rgba.z<<16u)|(rgba.w<<24u);
}
