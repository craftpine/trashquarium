param([string]$BatchPath = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies @([System.Drawing.Bitmap].Assembly.Location, [System.Drawing.Rectangle].Assembly.Location) -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public static class TqSpriteAudit {
 public static object Inspect(string path) {
  using(var b=new Bitmap(path)) {
   var rect=new Rectangle(0,0,b.Width,b.Height);
   var data=b.LockBits(rect,ImageLockMode.ReadOnly,PixelFormat.Format32bppArgb);
   try {
    int rowBytes=Math.Abs(data.Stride);byte[] bytes=new byte[rowBytes*b.Height];Marshal.Copy(data.Scan0,bytes,0,bytes.Length);
    int minX=b.Width,minY=b.Height,maxX=-1,maxY=-1,edge=0;long transparent=0,partial=0;
    for(int y=0;y<b.Height;y++)for(int x=0;x<b.Width;x++) {
     byte a=bytes[y*rowBytes+x*4+3];
     if(a==0)transparent++;else if(a<255)partial++;
     if(a>=128){minX=Math.Min(minX,x);minY=Math.Min(minY,y);maxX=Math.Max(maxX,x);maxY=Math.Max(maxY,y);if(x==0||y==0||x==b.Width-1||y==b.Height-1)edge++;}
    }
    return new {width=b.Width,height=b.Height,transparentPixels=transparent,partiallyTransparentPixels=partial,opaqueEdgePixels=edge,leftMargin=minX,topMargin=minY,rightMargin=b.Width-1-maxX,bottomMargin=b.Height-1-maxY,cornerAlpha=new int[]{bytes[3],bytes[(b.Width-1)*4+3],bytes[(b.Height-1)*rowBytes+3],bytes[(b.Height-1)*rowBytes+(b.Width-1)*4+3]}};
   }finally{b.UnlockBits(data);}
  }
 }
}
'@
$tqManifest=Get-Content -Raw -LiteralPath (Join-Path $BatchPath 'manifest.json') | ConvertFrom-Json
$tqRecords=@($tqManifest.assets | ForEach-Object {
 $tqFile=Join-Path $BatchPath $_.file
 if(Test-Path -LiteralPath $tqFile){
  [pscustomobject]@{id=$_.id;file=$_.file;sha256=(Get-FileHash -LiteralPath $tqFile -Algorithm SHA256).Hash;image=[TqSpriteAudit]::Inspect($tqFile)}
 }else{[pscustomobject]@{id=$_.id;file=$_.file;missing=$true}}
})
$tqRecords | ConvertTo-Json -Depth 8

