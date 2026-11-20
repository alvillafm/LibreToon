
@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read> params: array<u32>;
@group(0) @binding(2) var<storage, read> normals: array<vec2<f32>>;
@group(0) @binding(3) var<storage, read> mask: array<u32>;
@group(0) @binding(4) var<storage, read_write> result: array<u32>;

fn f(index: u32) -> f32 { return bitcast<f32>(params[index]); }
fn lerp2(a: vec2<f32>, b: vec2<f32>, t: f32) -> vec2<f32> { return a + (b - a) * t; }
fn slope(px: u32, py: u32) -> vec2<f32> {
    let w = params[8];
    let h = params[9];
    if w == 0u || h == 0u { return vec2<f32>(0.0); }
    let step = params[10];
    let x = px / step;
    let y = py / step;
    if step == 1u { return normals[y * w + x]; }
    let x1 = min(x + 1u, w - 1u);
    let y1 = min(y + 1u, h - 1u);
    let fx = f32(px % step) / f32(step);
    let fy = f32(py % step) / f32(step);
    return lerp2(lerp2(normals[y * w + x], normals[y * w + x1], fx),
                 lerp2(normals[y1 * w + x], normals[y1 * w + x1], fx), fy);
}
fn gobo(along: f32, across: f32) -> f32 {
    let x = across + f(36u);
    switch params[13] {
        case 1u: { return select(0.12, 1.0, fract(x * 9.0) < 0.55); }
        case 2u: { return select(1.0, 0.08, abs(x) < 0.035 || abs(along - 0.52) < 0.035); }
        case 3u: {
            let pattern = sin(x * 67.0 + along * 33.0) * sin(along * 54.0 - x * 24.0);
            return clamp(0.65 + pattern * 0.43, 0.12, 1.0);
        }
        case 4u: {
            if params[14] == 0u { return 1.0; }
            let uv = vec2<f32>((x + 0.5) * 47.0, along * 47.0);
            if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(47.0)) { return 0.0; }
            let ix = min(u32(uv.x), 46u);
            let iy = min(u32(uv.y), 46u);
            let t = uv - vec2<f32>(f32(ix), f32(iy));
            let a = f32(params[48u + iy * 48u + ix]);
            let b = f32(params[48u + iy * 48u + ix + 1u]);
            let c = f32(params[48u + (iy + 1u) * 48u + ix]);
            let d = f32(params[48u + (iy + 1u) * 48u + ix + 1u]);
            return ((a + (b-a) * t.x) + ((c + (d-c) * t.x) - (a + (b-a) * t.x)) * t.y) / 255.0;
        }
        default: { return 1.0; }
    }
}
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) pos: vec3<u32>) {
    let w = params[0];
    let h = params[1];
    if pos.x >= w || pos.y >= h { return; }
    let index = pos.y * w + pos.x;
    let packed = source[index];
    result[index] = packed;
    if pos.x < params[2] || pos.y < params[3] || pos.x >= params[4] || pos.y >= params[5] { return; }
    let alpha = f32(packed >> 24u);
    let kind = params[6];
    if alpha == 0.0 && kind != 8u && kind != 11u { return; }
    if kind >= 7u {
        let rgb=vec3<f32>(f32(packed&255u),f32((packed>>8u)&255u),f32((packed>>16u)&255u));
        var out=rgb;var out_alpha=alpha;
        if kind==7u {
            let luma=dot(rgb/max(alpha,1.0),vec3<f32>(0.2126,0.7152,0.0722));
            let t=clamp((luma-f(16u))/max(1.0-f(16u),0.01),0.0,1.0);
            let strength=t*t*(3.0-2.0*t);out=rgb*strength;out_alpha=alpha*strength;
        }else if kind==8u {
            let glow=mask[index];let gain=f(16u);let ga=f32(glow>>24u)*gain;
            out_alpha=min(alpha+ga*(1.0-alpha/255.0),255.0);
            out=min(rgb+vec3<f32>(f32(glow&255u),f32((glow>>8u)&255u),f32((glow>>16u)&255u))*gain,vec3<f32>(out_alpha));
        }else if kind==9u {
            let low=mask[index];let low_alpha=max(f32(low>>24u),1.0);
            let low_rgb=vec3<f32>(f32(low&255u),f32((low>>8u)&255u),f32((low>>16u)&255u))/low_alpha;
            out=clamp(rgb/alpha+f(16u)*(rgb/alpha-low_rgb),vec3<f32>(0.0),vec3<f32>(1.0))*alpha;
        }else if kind==11u {
            let old=mask[index];let original=vec4<f32>(f32(old&255u),f32((old>>8u)&255u),f32((old>>16u)&255u),f32(old>>24u));
            let blended=original+(vec4<f32>(rgb,alpha)-original)*f(16u);out=blended.xyz;out_alpha=blended.w;
        }else if kind==10u {
            let delta=vec2<f32>(f(16u),f(17u));let shift=vec2<i32>(sign(delta)*floor(abs(delta)+vec2<f32>(0.5)));
            let p=vec2<i32>(pos.xy)+shift;var coverage=0.0;
            if all(p>=vec2<i32>(0))&&p.x<i32(w)&&p.y<i32(h){coverage=f32(mask[u32(p.y)*w+u32(p.x)]>>24u)/255.0;}
            let depth=(1.0-coverage)*f(18u);let tint=vec3<f32>(f(28u),f(29u),f(30u));
            out=rgb*(vec3<f32>(1.0)-depth*(vec3<f32>(1.0)-tint));
        }
        let rgba=vec4<u32>(floor(vec4<f32>(out,out_alpha)+vec4<f32>(0.5)));
        result[index]=rgba.x|(rgba.y<<8u)|(rgba.z<<16u)|(rgba.w<<24u);return;
    }
    if kind == 6u {
        let center=vec2<f32>(f(19u),f(20u));
        let p=(vec2<f32>(pos.xy)-vec2<f32>(f(16u),f(17u))-center)/f(18u)+center;
        let xy=vec2<i32>(sign(p)*floor(abs(p)+vec2<f32>(0.5)));
        if any(xy<vec2<i32>(0)) || xy.x>=i32(w) || xy.y>=i32(h) {return;}
        let darkness=f32(mask[u32(xy.y)*w+u32(xy.x)]>>24u)/255.0*f(21u);
        var output=packed & 0xff000000u;
        for(var channel=0u;channel<3u;channel++) {
            let value=f32((packed>>(channel*8u)) & 255u);
            output|=u32(floor(value*(1.0-darkness)+0.5))<<(channel*8u);
        }
        result[index]=output;return;
    }
    var direction = vec3<f32>(f(19u), f(20u), f(21u));
    var attenuation = 1.0;
    if kind == 0u || kind == 5u {
        let delta = (vec2<f32>(f(16u), f(17u)) - vec2<f32>(pos.xy)) / f(18u);
        let d2 = delta.x * delta.x + delta.y * delta.y;
        if d2 >= 1.0 { return; }
        let z = f(21u) * 0.85;
        let inverse = 1.0 / sqrt(d2 + z * z);
        direction = vec3<f32>(delta * inverse, z * inverse);
        attenuation = pow(1.0 - sqrt(d2), f(27u));
    }
    if kind == 5u {
        let aim = vec2<f32>(f(32u), f(33u));
        let ray = (vec2<f32>(pos.xy) - vec2<f32>(f(16u), f(17u))) / f(18u);
        let along = dot(ray, aim);
        let across = dot(ray, vec2<f32>(-aim.y, aim.x));
        let cosine = along / max(length(ray), 0.00001);
        let edge = clamp((cosine - f(34u)) / f(35u), 0.0, 1.0);
        attenuation *= edge * edge * (3.0 - 2.0 * edge) * gobo(along, across);
        if attenuation <= 0.0 { return; }
    }
    let n = slope(pos.x, pos.y) * f(22u) * 3.2;
    let nz = 1.0 / sqrt(1.0 + dot(n, n));
    let ndot = n.x * direction.x + n.y * direction.y;
    var diffuse = clamp((ndot + direction.z) * nz, 0.0, 1.0);
    if kind == 2u { diffuse = 0.65; }
    if kind == 3u { diffuse = max(ndot * nz, 0.0) * sqrt(1.0 - nz); }
    if kind == 4u {
        let strength = max(-ndot * nz, 0.0) * sqrt(1.0 - nz) * 3.0;
        if f(31u) == 0.0 {
            diffuse = select(0.0, 1.0, strength >= 0.23);
        } else {
            diffuse = clamp((strength - 0.23) / f(31u) + 0.5, 0.0, 1.0) * min(strength * 3.0, 1.0);
        }
    }
    if params[11] >= 2u {
        let steps = f32(params[11] - 1u);
        let level = diffuse * steps;
        let blend = clamp((fract(level) - 0.40) / 0.20, 0.0, 1.0);
        diffuse = (floor(level) + blend * blend * (3.0 - 2.0 * blend)) / steps;
    }
    var specular = 0.0;
    if f(25u) > 0.0 && (kind == 0u || kind == 1u || kind == 5u) {
        let halfway = sqrt(2.0 + 2.0 * direction.z);
        let power = clamp(((ndot + direction.z + 1.0) * nz) / halfway, 0.0, 1.0);
        specular = pow(power, f(26u)) * f(25u);
    }
    var influence = 1.0;
    if params[7] != 0u {
        influence = f32(mask[index] >> 24u) / 255.0;
        if params[7] == 2u { influence = 1.0 - influence; }
    }
    let strength = f(23u) * attenuation * f(37u) * influence;
    var shade = 0.0;
    if kind == 0u || kind == 1u || kind == 5u { shade = (1.0 - diffuse) * f(24u) * strength; }
    var output = packed & 0xff000000u;
    for (var channel = 0u; channel < 3u; channel++) {
        let value = f32((packed >> (channel * 8u)) & 255u);
        let tint = f(28u + channel);
        var lit = value * (1.0 - shade)
            + ((alpha - value) * diffuse * 0.64 + alpha * specular) * strength * tint;
        if kind == 4u {
            var ink = 1.0;
            if params[12] != 0u { ink = min(value / alpha * 6.0, 1.0); }
            let weight = clamp(diffuse * strength * ink, 0.0, 1.0);
            lit = value * (1.0 - weight * (1.0 - tint));
        }
        output |= u32(floor(clamp(lit, 0.0, alpha) + 0.5)) << (channel * 8u);
    }
    result[index] = output;
}
