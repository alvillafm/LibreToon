@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read> params: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@group(0) @binding(3) var<storage, read> auxiliary: array<u32>;
fn f(i: u32) -> f32 { return bitcast<f32>(params[i]); }
fn unpack(v: u32) -> vec4<f32> { return vec4<f32>(f32(v & 255u), f32((v >> 8u) & 255u), f32((v >> 16u) & 255u), f32(v >> 24u)); }
fn pack(v: vec4<f32>) -> u32 {
    let p = vec4<u32>(floor(clamp(v, vec4<f32>(0.0), vec4<f32>(255.0)) + vec4<f32>(0.5)));
    return p.x | (p.y << 8u) | (p.z << 16u) | (p.w << 24u);
}
fn lut(v: f32, channel: u32) -> f32 {
    let x = clamp(v, 0.0, 1.0) * 1024.0;
    let i = min(u32(x), 1023u);
    let a = f(16u + channel * 1025u + i);
    let b = f(17u + channel * 1025u + i);
    return a + (b-a) * (x-f32(i));
}
fn sample_at(p: vec2<i32>) -> vec4<f32> {
    if any(p < vec2<i32>(0)) || p.x >= i32(params[0]) || p.y >= i32(params[1]) { return vec4<f32>(0.0); }
    return unpack(source[u32(p.y) * params[0] + u32(p.x)]);
}
fn round_signed(v: vec2<f32>) -> vec2<i32> { return vec2<i32>(sign(v) * floor(abs(v) + vec2<f32>(0.5))); }
fn hue(rgb: vec3<f32>, h_shift: f32, saturation: f32, light: f32) -> vec3<f32> {
    let hi = max(max(rgb.x,rgb.y),rgb.z); let lo = min(min(rgb.x,rgb.y),rgb.z);
    let delta = hi-lo; let old_l = (hi+lo)*0.5;
    var s = 0.0; var h = 0.0;
    if delta > 0.00001 {
        s = delta / max(1.0 - abs(2.0 * old_l - 1.0),0.00001);
        if hi == rgb.x { h = (rgb.y-rgb.z)/delta; }
        else if hi == rgb.y { h = 2.0+(rgb.z-rgb.x)/delta; }
        else { h = 4.0+(rgb.x-rgb.y)/delta; }
    }
    h = h + h_shift/60.0; h = h - floor(h/6.0)*6.0;
    s = clamp(s*(1.0+saturation),0.0,1.0); let l = clamp(old_l+light,0.0,1.0);
    let chroma = (1.0-abs(2.0*l-1.0))*s;
    let h2 = h - floor(h/2.0)*2.0; let x = chroma*(1.0-abs(h2-1.0));
    var v = vec3<f32>(chroma,0.0,x);
    switch u32(h) {
        case 0u: { v=vec3<f32>(chroma,x,0.0); } case 1u: {v=vec3<f32>(x,chroma,0.0);}
        case 2u: {v=vec3<f32>(0.0,chroma,x);} case 3u: {v=vec3<f32>(0.0,x,chroma);}
        case 4u: {v=vec3<f32>(x,0.0,chroma);} default: {}
    }
    return v + vec3<f32>(l-chroma/2.0);
}
@compute @workgroup_size(16,16)
fn main(@builtin(global_invocation_id) p: vec3<u32>) {
    let w=params[0]; let h=params[1]; if p.x>=w || p.y>=h {return;}
    let i=p.y*w+p.x;
    if params[2]==16u{output[i]=source[i*2u];return;}
    let original=source[i]; let pixel=unpack(original); let alpha=pixel.w;
    let kind=params[2]; let a=f(4u); let b=f(5u); let c=f(6u);
    if kind==12u {
        let at=(p.y+params[5])*params[4]+p.x+params[5];
        var packed=0u;
        if params[3]==1u {packed=auxiliary[at];}else{packed=params[16u+at];}
        let foreground=unpack(packed);
        output[i]=pack(min(foreground+pixel*(1.0-foreground.w/255.0),vec4<f32>(255.0)));return;
    }
    if kind==13u {output[i]=pack(vec4<f32>(pixel.xyz+vec3<f32>(255.0-alpha),255.0));return;}
    if kind==15u {output[i]=original;return;}
    if kind==7u {
        let angle=b*0.017453292519943295; let direction=vec2<f32>(cos(angle),sin(angle));
        let distance=f(7u); var sum=vec4<f32>(0.0);
        for(var step=-4;step<=4;step++) {sum+=sample_at(vec2<i32>(p.xy)+round_signed(direction*(distance*f32(step)/8.0)));}
        output[i]=pack(sum/9.0); return;
    }
    if alpha==0.0 {output[i]=original;return;}
    let rgb=pixel.xyz/alpha; let tint=vec3<f32>(f(8u),f(9u),f(10u)); var out=rgb;
    switch kind {
        case 0u: {out=hue(rgb,a,b,c);}
        case 1u: {out=vec3<f32>(lut(lut(rgb.x,1u),0u),lut(lut(rgb.y,2u),0u),lut(lut(rgb.z,3u),0u));}
        case 2u: {
            let xy=2.0*vec2<f32>(p.xy)/vec2<f32>(f32(w),f32(h))-vec2<f32>(1.0);
            let t=clamp((length(xy)/1.4142135623730951-b)/max(c,0.01),0.0,1.0);
            let strength=t*t*(3.0-2.0*t)*a; out=rgb+(tint-rgb)*strength;
        }
        case 3u: {
            let y=dot(rgb,vec3<f32>(0.2126,0.7152,0.0722));
            let mapped=clamp(tint+vec3<f32>((y-f(11u))*b),vec3<f32>(0.0),vec3<f32>(1.0));
            out=rgb+(mapped-rgb)*a;
        }
        case 4u: {out=(rgb*f(7u)-vec3<f32>(0.5))*(1.0+b)+vec3<f32>(0.5);}
        case 5u: {let steps=floor(clamp(a,2.0,32.0)+0.5)-1.0;out=floor(rgb*steps+vec3<f32>(0.5))/steps;}
        case 6u: {
            let cell=max(b*f(7u),0.001); let xy=vec2<u32>(vec2<f32>(p.xy)/cell);
            var n=xy.x*0x9e3779b9u ^ xy.y*0x85ebca6bu ^ params[3]*0x27d4eb2du;
            n=n ^ (n>>16u);n=n*0x7feb352du;n=n ^ (n>>15u);
            let noise=(f32(n>>16u)/65535.0*2.0-1.0)*a*alpha*0.35;
            output[i]=pack(vec4<f32>(clamp(pixel.xyz+vec3<f32>(noise),vec3<f32>(0.0),vec3<f32>(alpha)),alpha));return;
        }
        case 8u: {
            let angle=b*0.017453292519943295;let direction=vec2<f32>(cos(angle),sin(angle));
            let red=sample_at(vec2<i32>(p.xy)+round_signed(direction*(-f(7u))));
            let green=sample_at(vec2<i32>(p.xy));
            let blue=sample_at(vec2<i32>(p.xy)+round_signed(direction*f(7u)));
            var r=0.0;var g=0.0;var z=0.0;
            if red.w>0.0 {r=red.x/red.w;}
            if green.w>0.0 {g=green.y/green.w;}
            if blue.w>0.0 {z=blue.z/blue.w;}
            out=rgb+(vec3<f32>(r,g,z)-rgb)*clamp(c,0.0,1.0);
        }
        default: {}
    }
    output[i]=pack(vec4<f32>(clamp(out,vec3<f32>(0.0),vec3<f32>(1.0))*alpha,alpha));
}
