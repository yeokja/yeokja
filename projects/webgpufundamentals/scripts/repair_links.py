"""Corrections verified against the pinned upstream tree; no guessed targets."""
import html
import re
from urllib.parse import urlsplit, urlunsplit

RENAMES = {
    'webgpu-lighitng-spot.html': 'webgpu-lighting-spot.html',
    'webgpu-3d-lighting-spot.html': 'webgpu-lighting-spot.html',
    'webpgu-textures.html': 'webgpu-textures.html',
    'webgpu-scene-graphics.html': 'webgpu-scene-graphs.html',
    'webgpu-orthograpic-projection.html': 'webgpu-orthographic-projection.html',
    'webgpu-orthograph-projection.html': 'webgpu-orthographic-projection.html',
    'webgpu-orthographic.html': 'webgpu-orthographic-projection.html',
    'webgpu-persective-projection.html': 'webgpu-perspective-projection.html',
    'webgpu-perspective.html': 'webgpu-perspective-projection.html',
    'webgpu-inter-stage-varaibles.html': 'webgpu-inter-stage-variables.html',
    'webgpu-compute-shaders-historgram.html': 'webgpu-compute-shaders-histogram.html',
    'webgpu-3dluts.html': 'webgpu-3dlut.html',
    'webgpu-shadow-maps.html': 'webgpu-shadows.html',
    'webgpu-import-textures.html': 'webgpu-importing-textures.html',
    'webgpu-cubemaps.html': 'webgpu-cube-maps.html',
    'webgpu-lighting-direction.html': 'webgpu-lighting-directional.html',
    'webgpu-primitives.html': 'webpgu-primitives.html',
    'webgpu-compute-shaders.md': 'webgpu-compute-shaders.html',
    'webgpu-memory-layout.md': 'webgpu-memory-layout.html',
    'webgpu-cameras': 'webgpu-cameras.html',
    'webgpu-vertex-buffers': 'webgpu-vertex-buffers.html',
}
MISSING_ARTICLES = {'webgpu-normal-mapping.html', 'webgpu-skinning.html', 'webgpu-blend-targets.html'}
MALFORMED_ANCHORS = {'a-texture', 'a-race-conditions', 'a-alphamode', 'a-discard', 'a-blending'}
WGSL_DEFINITIONS = {'compute-shader-grid', 'dispatch-size', 'global-invocation-id', 'local-invocation-id', 'local-invocation-index', 'workgroup-grid', 'workgroup-id'}


def repair_lesson_html(text, filename, language):
    def repair_anchor(match):
        before, url, after, label = match.groups()
        parts = urlsplit(html.unescape(url))
        if parts.scheme or parts.netloc:
            return match[0]
        basename = parts.path.rsplit('/', 1)[-1]
        if basename in MALFORMED_ANCHORS and not label.strip():
            return f'<a id="{basename}"></a>'
        if basename in MISSING_ARTICLES:
            note = '원문 준비 중' if language == 'ko' else 'Not yet available upstream'
            return f'<span class="upstream-unavailable" data-upstream-href="{html.escape(url, quote=True)}" title="{note}">{label} <small>({note})</small></span>'
        path = RENAMES.get(basename, parts.path)
        if filename == 'webgpu-compute-shaders-histogram.html' and basename == 'webgpu-compute-shaders.html':
            path = basename
        if basename == 'webgpu-vertex-buffers-instanced-colors':
            path = parts.path + '.html'
        if filename == 'webgpu-wgsl.html' and not parts.path and parts.fragment in WGSL_DEFINITIONS:
            url = 'https://www.w3.org/TR/WGSL/#' + parts.fragment
        else:
            url = urlunsplit(('', '', path, parts.query, parts.fragment))
        return f'<a{before}href="{html.escape(url, quote=True)}"{after}>{label}</a>'

    text = re.sub(r'<a\b([^>]*?)href="([^"]*)"([^>]*)>(.*?)</a>', repair_anchor, text, flags=re.S)
    if filename == 'webgpu-transparency.html' and 'id="copyExternalImageToTexture"' not in text:
        def add_example_anchor(match):
            block = match[0]
            if 'copySourceToTexture' in block and 'premultipliedAlpha' in block and 'copyExternalImageToTexture' in block:
                return '<span id="copyExternalImageToTexture"></span>' + block
            return block
        text = re.sub(r'<pre\b[^>]*>.*?</pre>', add_example_anchor, text, flags=re.S)
    if filename == 'webgpu-rasterization.html':
        text = re.sub(r'<link\b[^>]*href="[^"]*webgpu-rasterization\.css"[^>]*>', '', text)
        text = re.sub(r'<script\b[^>]*src="[^"]*webgpu-rasterization\.js"[^>]*>\s*</script>', '', text)
        note = '이 도표는 원문에서 아직 준비 중입니다.' if language == 'ko' else 'This diagram is not yet available in the upstream article.'
        text = re.sub(r'(<div\b[^>]*data-diagram="clip-space-to-texels"[^>]*>).*?(</div>)', lambda m: m[1] + note + m[2], text, flags=re.S)
    return text
