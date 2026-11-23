"""Build small animations and explanatory SVGs from headless manual captures.

Requires Pillow. Run from the repository root after manual_visuals' opt-in generator.
No screen capture, desktop automation, remote images, or user projects are used.
"""
from pathlib import Path
from html import escape
import base64

from PIL import Image

IMAGES = Path(__file__).resolve().parents[2] / "manual" / "assets" / "images"


def animation(name, sources, durations):
    frames = []
    for source in sources:
        with Image.open(IMAGES / source) as im:
            frames.append(im.resize((640, 360), Image.Resampling.LANCZOS).convert("RGB"))
    # A shared palette avoids color flicker between independently quantized frames.
    palette = frames[0].quantize(colors=128)
    frames = [frame.quantize(palette=palette) for frame in frames]
    frames[0].save(
        IMAGES / name, save_all=True, append_images=frames[1:],
        duration=durations, loop=0, optimize=True, disposal=2,
    )


def text(x, y, value, size=22, color="#e7ebf5", anchor="start"):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" text-anchor="{anchor}">{escape(value)}</text>'


def svg(name, title, description, body, height=420):
    (IMAGES / name).write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 {height}" role="img" aria-labelledby="title desc">'
        f'<title id="title">{escape(title)}</title><desc id="desc">{escape(description)}</desc>'
        '<defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto">'
        '<path d="M0 0 L0 6 L8 3Z" fill="#82b7ff"/></marker></defs>'
        f'<rect width="1200" height="{height}" rx="20" fill="#202738"/>'
        '<g font-family="system-ui, sans-serif">'
        + text(36, 48, title, 28) + text(36, 80, "EXPLANATORY DIAGRAM · not an application screenshot", 15, "#a5b5ce")
        + body + '</g></svg>', encoding="utf-8",
    )


def timeline():
    body = ""
    for frame in range(1, 25):
        x = 48 + (frame - 1) * 46
        color = "#345f81" if frame < 9 or frame >= 13 else ("#82714c" if frame < 11 else "#97546b")
        body += f'<rect x="{x}" y="142" width="42" height="64" rx="5" fill="{color}"/>'
        body += text(x + 21, 130, str(frame), 16, anchor="middle")
        if frame in (1, 9, 11, 13):
            body += f'<circle cx="{x+21}" cy="174" r="7" fill="#ffffff"/>'
    for x, width, label, color in [(48, 364, "OPEN · hold", "#345f81"), (416, 88, "HALF", "#82714c"), (508, 88, "CLOSED", "#97546b"), (600, 548, "OPEN · hold", "#345f81")]:
        body += f'<rect x="{x}" y="225" width="{width}" height="44" rx="7" fill="{color}"/>'
        body += text(x+width/2, 253, label, 16, anchor="middle")
    body += text(48, 320, "White dot = a new drawing key. Other cells hold the preceding drawing.", 22)
    body += text(48, 360, "At 24 fps, this 24-frame exercise lasts one second.", 20, "#a5b5ce")
    svg("blink-timing.svg", "The four keys of a blink", "Open on frame 1, half closed on 9, closed on 11, open again on 13; exposures hold between keys.", body)


def depth():
    body = ""
    for x, y, color, title, value in [
        (110, 130, "#344e68", "BACKGROUND", "Distance 1500"),
        (430, 165, "#8d6149", "OWL", "Distance 1000"),
        (750, 200, "#245650", "FOREGROUND", "Distance 650"),
    ]:
        body += f'<path d="M{x} {y} l240 -20 v150 l-240 20Z" fill="{color}" stroke="#8fa9c4" stroke-width="2"/>'
        body += text(x+120, y+52, title, 20, anchor="middle")
        body += text(x+120, y+90, value, 18, anchor="middle")
    body += '<path d="M150 345 H1060" stroke="#82b7ff" stroke-width="4" marker-end="url(#arrow)"/>'
    body += text(160, 385, "Further from camera", 19, "#a5b5ce")
    body += text(1050, 385, "Closer to camera", 19, "#a5b5ce", "end")
    svg("forest-depth.svg", "Three layers, three distances", "The example places distant trees at 1500, the owl at 1000 and foreground foliage at 650. This is a conceptual depth diagram.", body)


def binding():
    body = '<path d="M250 300 V155" stroke="#68d8ca" stroke-width="16" stroke-linecap="round"/>'
    body += '<path d="M730 160 L760 310" stroke="#82b7ff" stroke-width="16" stroke-linecap="round"/>'
    for x,y in [(250,300),(250,155),(730,160),(760,310)]:
        body += f'<circle cx="{x}" cy="{y}" r="12" fill="#202738" stroke="#e7ebf5" stroke-width="3"/>'
    body += '<path d="M270 160 H705" stroke="#82b7ff" stroke-width="3" stroke-dasharray="8 8" marker-end="url(#arrow)"/>'
    body += '<path d="M770 160 Q880 170 885 280" fill="none" stroke="#f1c278" stroke-width="4" marker-end="url(#arrow)"/>'
    body += text(250, 123, "BODY · root", 24, anchor="middle")
    body += text(750, 123, "WING · child", 24, anchor="middle")
    body += text(420, 200, "parent → child", 20, "#a5b5ce")
    body += text(905, 235, "Pose this bone", 20, "#f1c278")
    body += text(250, 355, "Body symbol follows Body", 20, anchor="middle")
    body += text(780, 355, "Wing symbol follows Wing", 20, anchor="middle")
    svg("wing-binding.svg", "A tiny rig you can understand", "A torso root is the parent of a wing bone. Each symbol has a rigid binding to its corresponding bone. Pose the child to move only the wing.", body)


def annotate(name, title, callouts):
    """Keep the original capture intact; store teaching callouts in an SVG overlay."""
    source = base64.b64encode((IMAGES / f"{name}.png").read_bytes()).decode("ascii")
    body = (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1000" role="img" aria-labelledby="title">'
        f'<title id="title">{escape(title)}</title>'
        '<defs><marker id="tip" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto">'
        '<path d="M0 0 L0 6 L7 3Z" fill="#ffe29b"/></marker></defs>'
        f'<image width="1600" height="1000" href="data:image/png;base64,{source}"/>'
    )
    for n, (x, y, target_x, target_y) in enumerate(callouts, 1):
        body += f'<path d="M{x} {y} L{target_x} {target_y}" stroke="#172231" stroke-width="7"/>'
        body += f'<path d="M{x} {y} L{target_x} {target_y}" stroke="#ffe29b" stroke-width="3" marker-end="url(#tip)"/>'
        body += f'<circle cx="{x}" cy="{y}" r="18" fill="#ffe29b" stroke="#172231" stroke-width="3"/>'
        body += f'<text x="{x}" y="{y+7}" text-anchor="middle" font-family="system-ui,sans-serif" font-size="22" font-weight="700" fill="#172231">{n}</text>'
    (IMAGES / f"{name}-guide.svg").write_text(body + '</svg>', encoding="utf-8")


def main():
    animation("owl-blink.gif", ["blink-0.png", "blink-8.png", "blink-10.png", "blink-8.png", "blink-12.png"], [700, 80, 80, 80, 700])
    animation("forest-camera.gif", ["forest-0.png", "forest-11.png", "forest-23.png", "forest-11.png"], [650, 350, 650, 350])
    animation("wing-wave.gif", ["wing-0.png", "wing-11.png", "wing-23.png"], [500, 500, 500])
    timeline()
    depth()
    binding()
    annotate("workspace-color", "Actual editor capture: 1 ellipse, 2 Color tab, 3 fill and stroke", [(104,422,22,422),(1178,137,1265,137),(1480,148,1565,182)])
    annotate("workspace-timeline", "Actual editor capture: 1 eye layer, 2 closed drawing key, 3 current frame", [(245,874,159,899),(700,865,615,898),(365,729,281,762)])
    annotate("workspace-camera", "Actual editor capture: 1 camera tool, 2 camera track, 3 camera properties", [(115,563,51,563),(450,861,480,899),(1510,205,1320,245)])
    annotate("workspace-bones", "Actual editor capture: 1 wing bone, 2 selected bone settings, 3 binding control", [(859,420,748,440),(1493,440,1322,473),(1502,735,1330,765)])
    print("Generated three animations, three diagrams and four annotated editor captures.")


if __name__ == "__main__":
    main()
