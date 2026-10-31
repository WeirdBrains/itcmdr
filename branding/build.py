import os, json, io, cairosvg
from PIL import Image, ImageDraw, ImageFont
R=os.path.dirname(os.path.abspath(__file__))
SH="M512 130 C420 190 320 215 210 225 Q200 226 200 237 L200 470 C200 660 330 800 512 890 C694 800 824 660 824 470 L824 237 Q824 226 814 225 C704 215 604 190 512 130 Z"
PR='<path d="M362 370 L500 484 L362 598" fill="none" stroke="{c}" stroke-width="48" stroke-linecap="round" stroke-linejoin="round"/><path d="M540 602 L664 602" stroke="{c}" stroke-width="42" stroke-linecap="round"/>'
DEFS='''<defs><linearGradient id="n" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0B2350"/><stop offset="1" stop-color="#0E3473"/></linearGradient>
<linearGradient id="t" gradientUnits="userSpaceOnUse" x1="0" y1="360" x2="0" y2="610"><stop offset="0" stop-color="#2EE6E0"/><stop offset="1" stop-color="#22D3D3"/></linearGradient>
<linearGradient id="w" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFFFFF"/><stop offset="1" stop-color="#E6ECF5"/></linearGradient>
<mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="1024" height="1024"><path d="{sh}" fill="#fff"/>{pr}</mask></defs>'''.format(sh=SH,pr=PR.format(c="#000"))
def g(s,inner):  # scale around centre (shield visual centre y=510)
    return f'<g transform="translate(512 512) scale({s}) translate(-512 -510)">{inner}</g>'
def svg(body): return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">{DEFS}{body}</svg>\n'
def shield(v,mono=False):
    if mono: return f'<rect width="1024" height="1024" fill="#fff" mask="url(#m)"/>'
    if v=="office": return f'<path d="{SH}" fill="url(#n)"/>'+PR.format(c="url(#t)")
    return f'<path d="{SH}" fill="url(#w)"/>'+PR.format(c="#0B2350")+'<circle cx="700" cy="370" r="0" fill="none"/>'
BG={"office":'#F3F2F2',"admin":'url(#n)'}
def tile(v,s=0.86,rx=224):
    return svg((f'<rect width="1024" height="1024" rx="{rx}" fill="{BG[v]}"/>' if rx is not None else f'<rect width="1024" height="1024" fill="{BG[v]}"/>')+g(s,shield(v)))
def mono(s): return svg(g(s,shield(None,True)))
def _raw(s,size):
    if 'mask="url(#m)"' in s:
        from PIL import ImageChops
        a=_raw(s.replace('<rect width="1024" height="1024" fill="#fff" mask="url(#m)"/>',f'<path d="{SH}" fill="#fff"/>'),size)
        b=_raw(s.replace('<rect width="1024" height="1024" fill="#fff" mask="url(#m)"/>',PR.format(c="#fff")),size)
        al=ImageChops.subtract(a.getchannel("A"),b.getchannel("A"))
        out=Image.new("RGBA",a.size,(255,255,255,0)); out.putalpha(al); return out
    return Image.open(io.BytesIO(cairosvg.svg2png(bytestring=s.encode(),output_width=size,output_height=size))).convert("RGBA")
def png(s,path,size,bg=None):
    os.makedirs(os.path.dirname(path),exist_ok=True)
    im=_raw(s,size)
    if bg: b=Image.new("RGBA",im.size,bg); b.alpha_composite(im); im=b
    im.save(path)
def w(path,s):
    os.makedirs(os.path.dirname(path),exist_ok=True); open(path,"w").write(s)
# shared mark
w(f"{R}/shared/logo-mark.svg",svg(g(1.0,shield("office"))))
w(f"{R}/shared/logo-mark-mono.svg",mono(1.0))
png(svg(g(1.0,shield("office"))),f"{R}/shared/png/logo-mark-1024.png",1024)
png(mono(1.0),f"{R}/shared/png/logo-mark-mono-1024.png",1024)
font="/usr/share/fonts/truetype/sand-box/google/Inter/Inter-VariableFont_opsz,wght.ttf"
for v in ["office","admin"]:
    d=f"{R}/{v}"; S={}
    S["icon"]=tile(v); w(f"{d}/svg/icon-{v}.svg",S["icon"])
    S["square"]=tile(v,0.80,None); w(f"{d}/svg/icon-{v}-fullbleed.svg",S["square"])
    # admin mono: shield with a bar badge? keep same glyph; distinguish via 3 dots under? use outline shield for admin mono
    if v=="office": S["mono"]=mono(0.86)
    else:
        S["mono"]=svg(g(0.86,f'<path d="{SH}" fill="none" stroke="#fff" stroke-width="56" stroke-linejoin="round"/>'+PR.format(c="#fff")))
    w(f"{d}/svg/icon-{v}-mono.svg",S["mono"])
    fg=svg(g(0.70,shield(v))); bgs=svg(f'<rect width="1024" height="1024" fill="{BG[v]}"/>')
    monofg=S["mono"].replace("scale(0.86)","scale(0.70)")
    w(f"{d}/svg/adaptive-foreground.svg",fg); w(f"{d}/svg/adaptive-background.svg",bgs); w(f"{d}/svg/adaptive-monochrome.svg",monofg)
    # Android
    for q,sz in {"mdpi":48,"hdpi":72,"xhdpi":96,"xxhdpi":144,"xxxhdpi":192}.items():
        png(S["icon"],f"{d}/android/mipmap-{q}/ic_launcher.png",sz)
        a=int(sz*108/48)
        png(fg,f"{d}/android/mipmap-{q}/ic_launcher_foreground.png",a)
        png(bgs,f"{d}/android/mipmap-{q}/ic_launcher_background.png",a)
        png(monofg,f"{d}/android/mipmap-{q}/ic_launcher_monochrome.png",a)
    for q,sz in {"mdpi":24,"hdpi":36,"xhdpi":48,"xxhdpi":72,"xxxhdpi":96}.items():
        png(S["mono"].replace("scale(0.86)","scale(0.98)"),f"{d}/android/drawable-{q}/ic_stat_notify.png",sz)
    w(f"{d}/android/mipmap-anydpi-v26/ic_launcher.xml",'<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n  <background android:drawable="@mipmap/ic_launcher_background"/>\n  <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n  <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>\n</adaptive-icon>\n')
    png(S["square"],f"{d}/android/play-store-512.png",512)
    # feature graphic
    fgimg=Image.new("RGB",(1024,500),"#0B2350" if v=="admin" else "#F3F2F2")
    dr=ImageDraw.Draw(fgimg)
    for y in range(500):
        if v=="admin": dr.line([(0,y),(1024,y)],fill=(11,35+int(17*y/500),80+int(35*y/500)))
    icon=Image.open(io.BytesIO(cairosvg.svg2png(bytestring=(fg if v=="office" else svg(g(0.86,shield(v)))).encode(),output_width=340,output_height=340)))
    fgimg.paste(icon,(70,80),icon)
    f=ImageFont.truetype(font,120); f.set_variation_by_axes([32,700]) if hasattr(f,"set_variation_by_axes") else None
    col="#0B2350" if v=="office" else "#FFFFFF"
    dr.text((440,250),"itcmdr",font=f,fill=col,anchor="lm")
    if v=="admin":
        f2=ImageFont.truetype(font,64); f2.set_variation_by_axes([32,500])
        tw=dr.textlength("itcmdr",font=f); dr.text((440+tw+24,262),"admin",font=f2,fill="#2EE6E0",anchor="lm")
    fgimg.save(f"{d}/android/feature-graphic-1024x500.png")
    # iOS
    ios=f"{d}/ios/AppIcon.appiconset"; os.makedirs(ios,exist_ok=True)
    png(S["square"],f"{d}/{v}-icon-1024.png",1024,"#F3F2F2" if v=="office" else "#0C2A5E")
    Image.open(f"{d}/{v}-icon-1024.png").convert("RGB").save(f"{d}/{v}-icon-1024.png")
    full=Image.open(f"{d}/{v}-icon-1024.png")
    imgs=[];spec=[("iphone",20,[2,3]),("iphone",29,[2,3]),("iphone",40,[2,3]),("iphone",60,[2,3]),("ipad",20,[1,2]),("ipad",29,[1,2]),("ipad",40,[1,2]),("ipad",76,[1,2]),("ipad",83.5,[2]),("ios-marketing",1024,[1])]
    for idi,pt,scs in spec:
        for sc in scs:
            px=int(pt*sc); fn=f"Icon-{pt:g}@{sc}x.png"
            if not os.path.exists(f"{ios}/{fn}"):
                cairosvg.svg2png(bytestring=S["square"].encode(),write_to=f"{ios}/{fn}",output_width=px,output_height=px)
                Image.open(f"{ios}/{fn}").convert("RGB").save(f"{ios}/{fn}")
            imgs.append({"idiom":idi,"size":f"{pt:g}x{pt:g}","scale":f"{sc}x","filename":fn})
    json.dump({"images":imgs,"info":{"version":1,"author":"xcode"}},open(f"{ios}/Contents.json","w"),indent=2)
    png(S["mono"].replace("scale(0.86)","scale(0.80)"),f"{d}/ios/AppIcon-tinted-1024.png",1024)
    # web
    png(S["icon"],f"{d}/web/favicon.png",32)
    for sz in (192,512):
        png(S["icon"],f"{d}/web/icons/Icon-{sz}.png",sz)
        png(svg(f'<rect width="1024" height="1024" fill="{BG[v]}"/>'+g(0.66,shield(v))),f"{d}/web/icons/Icon-maskable-{sz}.png",sz)
    # windows / mac / linux
    big=Image.open(io.BytesIO(cairosvg.svg2png(bytestring=S["icon"].encode(),output_width=256,output_height=256)))
    os.makedirs(f"{d}/windows",exist_ok=True)
    frames={s:Image.open(io.BytesIO(cairosvg.svg2png(bytestring=S["icon"].encode(),output_width=s,output_height=s))) for s in (16,24,32,48,64,256)}
    frames[256].save(f"{d}/windows/app_icon.ico",format="ICO",sizes=[(s,s) for s in frames],append_images=[frames[s] for s in (16,24,32,48,64)])
    mf={s:_raw(S["mono"].replace("scale(0.86)","scale(0.98)"),s) for s in (16,20,24,32,48,64)}
    mf[64].save(f"{d}/windows/tray_mono.ico",format="ICO",sizes=[(s,s) for s in mf])
    os.makedirs(f"{d}/macos",exist_ok=True)
    mac=svg(g(0.80,'')) # placeholder
    macsvg=svg(f'<g transform="translate(100 100) scale(0.8046875)"><rect width="1024" height="1024" rx="230" fill="{BG[v]}"/>{g(0.86,shield(v))}</g>')
    w(f"{d}/svg/icon-{v}-macos.svg",macsvg)
    m=Image.open(io.BytesIO(cairosvg.svg2png(bytestring=macsvg.encode(),output_width=1024,output_height=1024)))
    m.save(f"{d}/macos/AppIcon.icns")
    for s in (16,32,64,128,256,512,1024): m.resize((s,s),Image.LANCZOS).save(f"{d}/macos/icon_{s}.png") if False else cairosvg.svg2png(bytestring=macsvg.encode(),write_to=f"{d}/macos/icon_{s}.png",output_width=s,output_height=s)
    for s in (16,22,24,32,48,64,128,256,512): png(S["icon"],f"{d}/linux/hicolor/{s}x{s}/apps/itcmdr{'-admin' if v=='admin' else ''}.png",s)
    for s in (16,24,32,48,64,96): png(S["mono"].replace("scale(0.86)","scale(0.98)"),f"{d}/tray/tray-mono-{s}.png",s)
# preview

# preview
lf=ImageFont.truetype(font,40)
rows=[]
for v in ["office","admin"]:
    rows.append((v,tile(v))); rows.append((v+" mono",open(f"{R}/{v}/svg/icon-{v}-mono.svg").read()))
cols=[1024,180,48,32,16]; PW=260+sum(cols)+60*len(cols); CW=2*PW; CH=4*1064+20
Q=Image.new("RGB",(CW,CH),"#fff"); D=ImageDraw.Draw(Q)
for p,(bgc,tc) in enumerate([("#FFFFFF","#000"),("#121418","#ddd")]):
    D.rectangle([p*PW,0,p*PW+PW,CH],fill=bgc); y=20
    for name,s_ in rows:
        D.text((p*PW+20,y+500),name,font=lf,fill=tc); x=p*PW+260
        for sz in cols:
            im=_raw(s_,sz)
            if "mono" in name and bgc=="#FFFFFF":
                b=Image.new("RGBA",im.size,"#0E3473"); b.alpha_composite(im); im=b
            Q.paste(im,(x,y+(1024-sz)//2),im); x+=sz+60
        y+=1064
Q.save(f"{R}/preview.png")
