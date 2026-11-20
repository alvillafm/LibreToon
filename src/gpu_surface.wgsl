@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read> params:array<u32>;
@group(0) @binding(2) var<storage,read_write> field:array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> sites:array<u32>;
@group(0) @binding(4) var<storage,read_write> boundaries:array<f32>;
@group(0) @binding(5) var<storage,read_write> result:array<vec2<u32>>;
@group(0) @binding(6) var<storage,read> height_map:array<u32>;
fn stride()->u32{return params[2]+2u;}
fn at(x:u32,y:u32)->u32{return (y+1u)*stride()+x+1u;}
@compute @workgroup_size(16,16)
fn coverage(@builtin(global_invocation_id) p:vec3<u32>){
    let w=stride();let h=params[3]+2u;if p.x>=w||p.y>=h{return;}
    let x=u32(clamp(i32(p.x)-1,0,i32(params[2])-1));let y=u32(clamp(i32(p.y)-1,0,i32(params[3])-1));
    let step=params[4];var sums=vec4<u32>(0u);var count=0u;var height_sum=0.0;
    for(var py=y*step;py<min((y+1u)*step,params[1]);py++){
        for(var px=x*step;px<min((x+1u)*step,params[0]);px++){
            let i=py*params[0]+px;let v=source[i];sums+=vec4<u32>(v&255u,(v>>8u)&255u,(v>>16u)&255u,v>>24u);count++;
            if params[9]!=0u{let height=height_map[i];height_sum+=f32(height&255u)-f32((height>>16u)&255u);}
        }
    }
    let alpha=f32(sums.w);var coverage=f32(alpha)/(255.0*f32(count));
    if arrayLength(&params)>10u&&params[10]==1u{coverage=1.0-coverage;}
    let edge=coverage/max(1.0-coverage,0.00000011920928955078125);
    let luma=(0.2126*f32(sums.x)+0.7152*f32(sums.y)+0.0722*f32(sums.z))/max(alpha,1.0);
    let detail=luma*bitcast<f32>(params[8])*8.0+height_sum/(255.0*f32(count))*22.0;
    let far=f32(w*w+h*h);field[p.y*w+p.x]=vec4<f32>(coverage,detail,min(edge*edge,far),0.0);
}
fn value(line:u32,q:u32,vertical:bool)->f32{
    if vertical{return field[q*stride()+line].w;}return field[line*stride()+q].z;
}
fn intersection(line:u32,q:u32,p:u32,vertical:bool)->f32{
    return (value(line,q,vertical)-value(line,p,vertical))/(2.0*f32(q-p))+f32(q+p)*0.5;
}
fn distance_line(line:u32,vertical:bool){
    let length=select(stride(),params[3]+2u,vertical);let base=line*(length+1u);
    var last=0u;sites[base]=0u;boundaries[base]=-1e30;boundaries[base+1u]=1e30;
    for(var q=1u;q<length;q++){
        var cross=intersection(line,q,sites[base+last],vertical);
        while last>0u && cross<=boundaries[base+last]{last--;cross=intersection(line,q,sites[base+last],vertical);}
        last++;sites[base+last]=q;boundaries[base+last]=cross;boundaries[base+last+1u]=1e30;
    }
    var site=0u;
    for(var q=0u;q<length;q++){
        while boundaries[base+site+1u]<f32(q){site++;}
        let p=sites[base+site];let delta=abs(f32(q)-f32(p));let distance=delta*delta+value(line,p,vertical);
        if vertical{field[q*stride()+line].z=distance;}else{field[line*stride()+q].w=distance;}
    }
}
@compute @workgroup_size(64)
fn rows(@builtin(global_invocation_id) p:vec3<u32>){if p.x<params[3]+2u{distance_line(p.x,false);}}
@compute @workgroup_size(64)
fn columns(@builtin(global_invocation_id) p:vec3<u32>){if p.x<stride(){distance_line(p.x,true);}}
@compute @workgroup_size(16,16)
fn height_values(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[2]||p.y>=params[3]{return;}
    let i=at(p.x,p.y);let limit=max(bitcast<f32>(params[5])/f32(params[4]),1.0);
    let edge=max(sqrt(field[i].z)-0.5,0.0);let t=min(edge/limit,1.0);
    field[i].y=(t*(2.0-t)*min(limit,48.0)+field[i].y)*field[i].x;
}
fn sample_height(line:u32,position:u32,vertical:bool)->f32{
    if vertical{return field[at(line,min(position,params[3]-1u))].w;}
    return field[at(min(position,params[2]-1u),line)].y;
}
fn smooth_line(line:u32,vertical:bool){
    let length=select(params[2],params[3],vertical);let radius=params[6];let weight=1.0/f32(2u*radius+1u);
    var sum=f32(radius)*sample_height(line,0u,vertical);
    for(var i=0u;i<=radius;i++){sum+=sample_height(line,i,vertical);}
    for(var i=0u;i<length;i++){
        if vertical{field[at(line,i)].y=sum*weight;}else{field[at(i,line)].w=sum*weight;}
        sum+=sample_height(line,i+radius+1u,vertical)-sample_height(line,u32(max(i32(i)-i32(radius),0)),vertical);
    }
}
@compute @workgroup_size(64)
fn smooth_x(@builtin(global_invocation_id) p:vec3<u32>){if p.x<params[3]{smooth_line(p.x,false);}}
@compute @workgroup_size(64)
fn smooth_y(@builtin(global_invocation_id) p:vec3<u32>){if p.x<params[2]{smooth_line(p.x,true);}}
@compute @workgroup_size(16,16)
fn slopes(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[2]||p.y>=params[3]{return;}
    let left=field[at(u32(max(i32(p.x)-1,0)),p.y)].y;let right=field[at(min(p.x+1u,params[2]-1u),p.y)].y;
    let top=field[at(p.x,u32(max(i32(p.y)-1,0)))].y;let bottom=field[at(p.x,min(p.y+1u,params[3]-1u))].y;
    result[p.y*params[2]+p.x]=bitcast<vec2<u32>>(vec2<f32>(left-right,top-bottom)*0.5);
}
@compute @workgroup_size(16,16)
fn outline(@builtin(global_invocation_id) p:vec3<u32>){
    if p.x>=params[2]||p.y>=params[3]{return;}
    let index=p.y*params[2]+p.x;let original=source[index];let alpha=f32(original>>24u);
    let distance=sqrt(field[at(p.x,p.y)].z);
    let added=(255.0-alpha)*bitcast<f32>(params[13])*clamp((bitcast<f32>(params[11])+0.5-distance)/bitcast<f32>(params[12]),0.0,1.0);
    var color=u32(floor(alpha+added+0.5))<<24u;
    for(var channel=0u;channel<3u;channel++){
        let value=f32((original>>(channel*8u))&255u)+added*bitcast<f32>(params[14u+channel]);
        color|=u32(floor(value+0.5))<<(channel*8u);
    }
    result[index]=vec2<u32>(color,0u);
}
