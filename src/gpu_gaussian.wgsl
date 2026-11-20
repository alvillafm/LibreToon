@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read> params:array<u32>;
@group(0) @binding(2) var<storage,read_write> intermediate:array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> output:array<u32>;
fn unpack(v:u32)->vec4<f32>{return vec4<f32>(f32(v&255u),f32((v>>8u)&255u),f32((v>>16u)&255u),f32(v>>24u));}
@compute @workgroup_size(16,16)
fn vertical(@builtin(global_invocation_id) p:vec3<u32>){
    let w=params[0];let h=params[1];if p.x>=w||p.y>=h{return;}
    let radius=i32(params[2]/2u);var sum=vec4<f32>(0.0);
    for(var i=0u;i<params[2];i++){
        let y=u32(clamp(i32(p.y)+i32(i)-radius,0,i32(h)-1));
        sum+=unpack(source[y*w+p.x])*bitcast<f32>(params[4u+i]);
    }
    intermediate[p.y*w+p.x]=sum;
}
@compute @workgroup_size(16,16)
fn horizontal(@builtin(global_invocation_id) p:vec3<u32>){
    let w=params[0];let h=params[1];if p.x>=w||p.y>=h{return;}
    let radius=i32(params[2]/2u);var sum=vec4<f32>(0.0);
    for(var i=0u;i<params[2];i++){
        let x=u32(clamp(i32(p.x)+i32(i)-radius,0,i32(w)-1));
        sum+=intermediate[p.y*w+x]*bitcast<f32>(params[4u+i]);
    }
    let v=vec4<u32>(floor(clamp(sum,vec4<f32>(0.0),vec4<f32>(255.0))+vec4<f32>(0.5)));
    output[p.y*w+p.x]=v.x|(v.y<<8u)|(v.z<<16u)|(v.w<<24u);
}
