@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read> params:array<u32>;
@group(0) @binding(2) var<storage,read_write> intermediate:array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> output:array<u32>;
fn unpack(v:u32)->vec4<f32>{return vec4<f32>(f32(v&255u),f32((v>>8u)&255u),f32((v>>16u)&255u),f32(v>>24u));}
fn bounds(index:u32,old:u32,resized:u32)->vec2<u32>{
    let ratio=f32(old)/f32(resized);let center=(f32(index)+0.5)*ratio;let support=max(ratio,1.0);
    let left=u32(clamp(i32(floor(center-support)),0,i32(old)-1));let right=u32(clamp(i32(ceil(center+support)),i32(left)+1,i32(old)));
    return vec2<u32>(left,right);
}
fn weight(i:u32,index:u32,old:u32,resized:u32)->f32{
    let ratio=f32(old)/f32(resized);let center=(f32(index)+0.5)*ratio-0.5;
    return max(1.0-abs((f32(i)-center)/max(ratio,1.0)),0.0);
}
@compute @workgroup_size(16,16)
fn vertical(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[0]||p.y>=params[3]{return;}
    let b=bounds(p.y,params[1],params[3]);var total=0.0;
    for(var y=b.x;y<b.y;y++){total+=weight(y,p.y,params[1],params[3]);}
    var sum=vec4<f32>(0.0);for(var y=b.x;y<b.y;y++){sum+=unpack(source[y*params[0]+p.x])*(weight(y,p.y,params[1],params[3])/total);}
    intermediate[p.y*params[0]+p.x]=sum;
}
@compute @workgroup_size(16,16)
fn horizontal(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[2]||p.y>=params[3]{return;}
    let b=bounds(p.x,params[0],params[2]);var total=0.0;
    for(var x=b.x;x<b.y;x++){total+=weight(x,p.x,params[0],params[2]);}
    var sum=vec4<f32>(0.0);for(var x=b.x;x<b.y;x++){sum+=intermediate[p.y*params[0]+x]*(weight(x,p.x,params[0],params[2])/total);}
    let rgba=vec4<u32>(floor(clamp(sum,vec4<f32>(0.0),vec4<f32>(255.0))+vec4<f32>(0.5)));
    output[p.y*params[2]+p.x]=rgba.x|(rgba.y<<8u)|(rgba.z<<16u)|(rgba.w<<24u);
}
