$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies @([Drawing.Bitmap].Assembly.Location) -TypeDefinition @'
using System;
using System.Drawing;
public static class HuntPngAudit {
 public static object Read(string file) {
  using (var b = new Bitmap(file)) {
   long clear=0, partial=0; int edge=0,minX=b.Width,minY=b.Height,maxX=-1,maxY=-1;
   for(int y=0;y<b.Height;y++) for(int x=0;x<b.Width;x++) {
    int a=b.GetPixel(x,y).A;
    if(a==0)clear++; else if(a<255)partial++;
    if(a>=128) {minX=Math.Min(x,minX);minY=Math.Min(y,minY);maxX=Math.Max(x,maxX);maxY=Math.Max(y,maxY);if(x==0||y==0||x==b.Width-1||y==b.Height-1)edge++;}
   }
   return new {width=b.Width,height=b.Height,transparentPixels=clear,partiallyTransparentPixels=partial,opaqueEdgePixels=edge,leftMargin=minX,topMargin=minY,rightMargin=b.Width-1-maxX,bottomMargin=b.Height-1-maxY,cornerAlpha=new[]{b.GetPixel(0,0).A,b.GetPixel(b.Width-1,0).A,b.GetPixel(0,b.Height-1).A,b.GetPixel(b.Width-1,b.Height-1).A}};
  }
 }
}
'@
$root = Join-Path (Split-Path $PSScriptRoot -Parent) 'public/art/hunt'
@('boat.png','claw.png','shell_0.png','shell_1.png','shell_2.png') | ForEach-Object {
  $file = Join-Path $root $_
  [pscustomobject]@{file=$_;sha256=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash;image=[HuntPngAudit]::Read($file)}
} | ConvertTo-Json -Depth 5
