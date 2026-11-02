# Validate the exact exporter command contract without replacing bundled FFmpeg.
param(
    [Parameter(Mandatory)][string]$CandidateDirectory,
    [string]$Baseline = "assets/tools/ffmpeg.exe",
    [string]$ResultsDirectory = "work/ffmpeg-validation"
)
$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$candidate = [IO.Path]::GetFullPath((Join-Path $repo $CandidateDirectory))
$baselineExe = [IO.Path]::GetFullPath((Join-Path $repo $Baseline))
$resultDir = [IO.Path]::GetFullPath((Join-Path $repo $ResultsDirectory))
New-Item -ItemType Directory -Force $resultDir | Out-Null
$exe = Join-Path $candidate "ffmpeg.exe"
$probe = Join-Path $candidate "ffprobe.exe"
function Run([string]$program, [string[]]$arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new($program)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argValue in $arguments) { $info.ArgumentList.Add($argValue) }
    $process = [Diagnostics.Process]::Start($info)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(120000)) { $process.Kill($true); throw "FFmpeg validation timed out" }
    $text = $stdout.GetAwaiter().GetResult()
    $errors = $stderr.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 0) { throw "$program failed: $errors" }
    $process.Dispose()
    return $text
}
if (-not ("LibreToonFfmpegFixture" -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.IO;
public static class LibreToonFfmpegFixture {
 public static void Create(string path,int w,int h,int frames) {
  byte[] rgb=new byte[w*h*3*frames];
  int[,] colors={{230,30,20},{20,230,30},{30,20,230}};
  for(int f=0;f<frames;f++) for(int y=0;y<h;y++) for(int x=0;x<w;x++) {
   int k=((f*h+y)*w+x)*3;
   for(int c=0;c<3;c++) {
    int v=255;
    if(x>8&&x<w-8&&y>8&&y<h-8) v=colors[f%3,c];
    if(y>h*3/4&&x>8&&x<w-8) v=(x*7+y*3+c*71+f*13)%256;
    if(Math.Abs(x-w/2)<8&&Math.Abs(y-h/2)<8) v=colors[f%3,c];
    rgb[k+c]=(byte)v;
   }
  }
  File.WriteAllBytes(path,rgb);
 }
 public static double[] Compare(string source,string decoded,int w,int h,int ow,int oh,int frames) {
  byte[] a=File.ReadAllBytes(source),b=File.ReadAllBytes(decoded);
  if(b.Length!=ow*oh*frames*3) throw new Exception("Wrong decoded frame count or dimensions");
  double sum=0;
  for(int f=0;f<frames;f++) {
   int center=((f*oh+oh/2)*ow+ow/2)*3;
   if(b[center+f%3]<180||b[center+(f+1)%3]>90) throw new Exception("Wrong frame order or RGB channels");
   for(int y=0;y<oh;y++)for(int x=0;x<ow;x++)for(int c=0;c<3;c++){
    int v=(x<w&&y<h)?a[((f*h+y)*w+x)*3+c]:255;
    double d=v-b[((f*oh+y)*ow+x)*3+c];sum+=d*d;
   }
  }
  double mse=sum/b.Length;
  return new double[]{mse,mse==0?99:10*Math.Log10(255.0*255/mse)};
 }
 public static bool Faststart(string path) {
  byte[] a=File.ReadAllBytes(path);long pos=0,moov=-1,mdat=-1;
  while(pos+8<=a.Length) {
   int p=(int)pos;uint n=((uint)a[p]<<24)|((uint)a[p+1]<<16)|((uint)a[p+2]<<8)|a[p+3];
   string type=System.Text.Encoding.ASCII.GetString(a,p+4,4);
   if(type=="moov")moov=pos;if(type=="mdat")mdat=pos;
   if(n<8)break;pos+=n;
  }
  return moov>=0&&mdat>=0&&moov<mdat;
 }
 public static int GifLoop(string path) {
  byte[] a=File.ReadAllBytes(path);byte[] s=System.Text.Encoding.ASCII.GetBytes("NETSCAPE2.0");
  for(int i=0;i<a.Length-s.Length-5;i++) {
   bool equal=true;for(int j=0;j<s.Length;j++)if(a[i+j]!=s[j])equal=false;
   int p=i+s.Length;if(equal&&a[p]==3&&a[p+1]==1)return a[p+2]|a[p+3]<<8;
  }
  return -1;
 }
}
'@
}
$results = @()
foreach ($fixture in @(
    @{name="odd";w=193;h=109;frames=12;crf=18;colors=64;dither="sierra2_4a";loops=2},
    @{name="fullhd";w=1920;h=1080;frames=3;crf=32;colors=256;dither="none";loops=0}
)) {
    $raw = Join-Path $resultDir "$($fixture.name).rgb"
    [LibreToonFfmpegFixture]::Create($raw,$fixture.w,$fixture.h,$fixture.frames)
    foreach ($format in @("mp4","mkv","avi","mov","gif")) {
        $ow = if ($format -eq "gif") {$fixture.w} else {$fixture.w + $fixture.w % 2}
        $oh = if ($format -eq "gif") {$fixture.h} else {$fixture.h + $fixture.h % 2}
        $measures = @{}
        foreach ($implementation in @("baseline","candidate")) {
            $program = if ($implementation -eq "baseline") {$baselineExe} else {$exe}
            $path = Join-Path $resultDir "$($fixture.name)-$implementation.$format"
            $arguments = @("-hide_banner","-loglevel","error","-nostdin","-y","-f","rawvideo","-pixel_format","rgb24","-video_size","$($fixture.w)x$($fixture.h)","-framerate","24","-i",$raw,"-an")
            switch ($format) {
                {$_ -in @("mp4","mkv")} {
                    $arguments += @("-vf","pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white","-c:v","libx264","-pix_fmt","yuv420p","-preset","medium","-crf","$($fixture.crf)")
                    if ($format -eq "mp4") {$arguments += @("-movflags","+faststart")}
                }
                "avi" {$arguments += @("-vf","pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white","-c:v","mpeg4","-q:v","3")}
                "mov" {$arguments += @("-vf","pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white","-c:v","prores_ks","-profile:v","3","-pix_fmt","yuv422p10le")}
                "gif" {$arguments += @("-filter_complex","split[a][b];[a]palettegen=max_colors=$($fixture.colors):stats_mode=single[p];[b][p]paletteuse=new=1:dither=$($fixture.dither)","-loop","$($fixture.loops)")}
            }
            $null = Run $program ($arguments + @($path))
            $decoded = "$path.rgb"
            $decodeArgs = @("-hide_banner","-loglevel","error","-nostdin","-y")
            if ($format -eq "gif") {$decodeArgs += @("-ignore_loop","1")}
            $null = Run $exe ($decodeArgs + @("-i",$path,"-fps_mode","passthrough","-f","rawvideo","-pix_fmt","rgb24",$decoded))
            $scores = [LibreToonFfmpegFixture]::Compare($raw,$decoded,$fixture.w,$fixture.h,$ow,$oh,$fixture.frames)
            $measures[$implementation] = @{mse=$scores[0];psnr=$scores[1]}
            $meta = (Run $probe @("-v","error","-select_streams","v:0","-count_frames","-show_entries","stream=width,height,nb_read_frames,avg_frame_rate,pix_fmt,codec_name","-of","json",$path)) | ConvertFrom-Json
            $stream = $meta.streams[0]
            if ($stream.width -ne $ow -or $stream.height -ne $oh -or [int]$stream.nb_read_frames -ne $fixture.frames) {throw "Wrong stream dimensions/frame count: $path"}
            $fraction = $stream.avg_frame_rate.Split("/")
            $fps = [double]$fraction[0] / [double]$fraction[1]
            if ($format -ne "gif" -and [Math]::Abs($fps-24) -gt 0.001) {throw "Unexpected FPS: $fps"}
            if ($format -eq "gif" -and [Math]::Abs($fps-24) -gt 2) {throw "GIF frame timing outside centisecond tolerance: $fps"}
            if ($format -eq "mp4" -and -not [LibreToonFfmpegFixture]::Faststart($path)) {throw "MP4 faststart missing"}
            if ($format -eq "gif" -and [LibreToonFfmpegFixture]::GifLoop($path) -ne $fixture.loops) {throw "GIF loop count changed"}
        }
        # Same codec/settings, allowing minor numerical variation across encoder versions.
        $limit = [Math]::Max($measures.baseline.mse*1.15,$measures.baseline.mse+4)
        if ($measures.candidate.mse -gt $limit) {throw "Quality regression for $($fixture.name)/$format"}
        $results += @{fixture=$fixture.name;format=$format;width=$ow;height=$oh;frames=$fixture.frames;fps=24;settings=$fixture;quality=$measures;maximumAllowedMse=$limit}
    }
}
$receipt = @{
    schema=1;status="passed";binarySha256=(Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant()
    baselineSha256=(Get-FileHash -LiteralPath $baselineExe).Hash.ToLowerInvariant()
    testScriptSha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash.ToLowerInvariant()
    cases=$results;limitations="Synthetic fixtures, not an exhaustive visual/performance study of every project."
}
$receipt | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $candidate "EXPORT_VALIDATION.json") -Encoding utf8
Write-Output "Export contract passed: $($results.Count) format/fixture cases against bundled baseline."
