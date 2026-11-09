 
 
 
 
 
 
 
 
 
 
 
 
 

 
 
pub(crate) const MASK_VERTEX: &str = r#"#version 150
in vec3 Position;
uniform mat4 View;
uniform mat4 Projection;
out float viewDepth;
void main() {
    vec4 eye = View * vec4(Position, 1.0);
    viewDepth = -eye.z;
    gl_Position = Projection * eye;
}
"#;

 
 
 
pub(crate) const MASK_FRAGMENT: &str = r#"#version 150
uniform vec3 Tint;
uniform int Occlude;
uniform sampler2D SceneDepth;
uniform mat4 InverseProjection;
uniform vec4 Viewport;
in float viewDepth;
out vec4 fragColor;
void main() {
    if (viewDepth <= 0.05) discard;
    if (Occlude != 0) {
        vec2 screenUv = (gl_FragCoord.xy - Viewport.xy) / Viewport.zw;
        float depth = texture(SceneDepth, screenUv).r;
        vec4 scene = InverseProjection * vec4(screenUv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
        float sceneDepth = -scene.z / scene.w;
        if (viewDepth > sceneDepth + 0.05) discard;
    }
    fragColor = vec4(Tint, 1.0);
}
"#;

 
pub(crate) const POST_VERTEX: &str = r#"#version 150
out vec2 uv;
void main() {
    vec2 p = vec2(gl_VertexID == 1 ? 3.0 : -1.0, gl_VertexID == 2 ? 3.0 : -1.0);
    uv = (p + 1.0) * 0.5;
    gl_Position = vec4(p, 0.0, 1.0);
}
"#;

 
 
 
 
pub(crate) const DOWN_FRAGMENT: &str = r#"#version 150
in vec2 uv;
out vec4 fragColor;
uniform sampler2D Mask;
uniform vec2 srcTexel;
uniform float spread;
vec4 tap(vec2 p) {
    vec4 c = texture(Mask, p);
    float m = c.a * step(0.002, max(max(c.r, c.g), c.b));
    return vec4(c.rgb * m, m);
}
void main() {
    vec2 o = vec2(spread) * srcTexel;
    fragColor = 0.25 * (tap(uv + vec2(-o.x, -o.y)) + tap(uv + vec2(o.x, -o.y))
                      + tap(uv + vec2(-o.x,  o.y)) + tap(uv + vec2(o.x,  o.y)));
}
"#;

 
 
pub(crate) const BLUR_FRAGMENT: &str = r#"#version 150
in vec2 uv;
out vec4 fragColor;
uniform sampler2D Source;
uniform vec2 direction;
uniform float sigma;
void main() {
    vec4 acc = textureLod(Source, uv, 0.0);
    if (sigma < 0.3) { fragColor = acc; return; }
    float tw = 1.0;
    float k = -0.5 / (sigma * sigma);
    int n = min(int(ceil(sigma * 3.0)), 48);
    for (int i = 1; i <= n; i += 2) {
        float a = float(i);
        float b = a + 1.0;
        float wa = exp(a * a * k);
        float wb = (i + 1 <= n) ? exp(b * b * k) : 0.0;
        float w = wa + wb;
        float o = (a * wa + b * wb) / w;
        acc += (textureLod(Source, uv + direction * o, 0.0)
              + textureLod(Source, uv - direction * o, 0.0)) * w;
        tw += 2.0 * w;
    }
    fragColor = acc / tw;
}
"#;

 
 
 
pub(crate) const COMPOSITE_FRAGMENT: &str = r#"#version 150
in vec2 uv;
out vec4 fragColor;
uniform sampler2D Mask;
uniform sampler2D Glow;
uniform sampler2D Scene;
uniform vec2 texel;
uniform vec2 glowTexel;
uniform float pxScale;
uniform float time;
uniform float outline;
uniform float intensity;
uniform float fill;
uniform float tintAmount;
uniform float aspect;
uniform vec3 accent;
uniform vec3 accent2;
uniform float thinInner;
uniform float lineCount;
uniform float refraction;
uniform float dispersion;
uniform float thickness;
uniform float bass;
uniform float beatAge[4];
uniform float flash;
uniform int Mode;

float cov(vec2 p) {
    vec4 c = texture(Mask, p);
    return c.a * step(0.002, max(max(c.r, c.g), c.b));
}
vec3 hsv2rgb(vec3 c) {
    vec3 p = abs(fract(c.xxx + vec3(1.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0);
    return c.z * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), c.y);
}
float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
float hash11(float p) {
    p = fract(p * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}
vec3 playerColor(float m, vec4 g) {
    vec3 c = m > 0.01 ? texture(Mask, uv).rgb : g.rgb / max(g.a, 1e-4);
    float mx = max(max(c.r, c.g), c.b);
    return mx > 1e-3 ? c / mx * min(1.0, mx * 1.6 + 0.2) : vec3(1.0);
}
float edgeRing(float radiusPx) {
    float e = 0.0;
    for (int i = 0; i < 8; i++) {
        float a = float(i) * 0.7853982 + 0.3927;
        e += 1.0 - cov(uv + vec2(cos(a), sin(a)) * texel * radiusPx);
    }
    return e * 0.125;
}
float outerGlow(vec4 g) { return pow(clamp(g.a * 2.0, 0.0, 1.0), 0.6); }
float vnoise(vec2 p) {
    vec2 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1, 0)), f.x),
               mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), f.x), f.y);
}
float fbm(vec2 p) {
    float s = 0.0, a = 0.5;
    for (int i = 0; i < 4; i++) { s += a * vnoise(p); p = p * 2.03 + 7.1; a *= 0.5; }
    return s / 0.9375;
}
float fbm2(vec2 p) {
    float s = 0.0, a = 0.5;
    for (int i = 0; i < 4; i++) {
        s += a * vnoise(p);
        p = mat2(0.8, 0.6, -0.6, 0.8) * p * 2.03;
        a *= 0.5;
    }
    return s / 0.9375;
}
float stars(vec2 p, float cells, float t) {
    vec2 g = p * cells, id = floor(g), f = fract(g) - 0.5;
    float h = hash12(id);
    if (h < 0.86) return 0.0;
    vec2 o = (vec2(hash12(id + 7.1), hash12(id + 3.3)) - 0.5) * 0.6;
    float d = length(f - o);
    float tw = 0.6 + 0.4 * sin(t * (2.0 + 4.0 * h) + h * 40.0);
    return smoothstep(0.08, 0.0, d) * tw * (h - 0.86) * 7.0;
}
vec3 prismField(vec2 p, float t) {
    vec3 light = vec3(0.0);
    for (int i = 0; i < 4; i++) {
        float fi = float(i);
        vec2 c = vec2(sin(t * (0.21 + fi * 0.05) + fi * 1.7) * 0.55 * aspect,
                      cos(t * (0.17 + fi * 0.04) + fi * 2.3) * 0.38);
        float r = 0.30 + 0.1 * sin(t * 0.3 + fi);
        float g = exp(-dot(p - c, p - c) / (r * r));
        float pulse = 0.5 + 0.5 * sin(t * 1.4 + fi * 1.9);
        light += mix(accent, accent2, 0.5 + 0.5 * sin(fi * 1.9 + t * 0.35))
               * g * (0.4 + 0.6 * pulse);
    }
    float rays = 0.5 + 0.5 * sin((p.x + p.y * 0.4) * 9.0 + t * 0.6);
    return light * (0.6 + 0.3 * rays);
}
float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }
float iso(float v, float level, float w) {
    float fw = max(fwidth(v), 1e-4);
    return 1.0 - smoothstep(0.0, w * fw, abs(v - level));
}

void main() {
    float m = cov(uv);
    vec4 g = textureLod(Glow, uv, 0.0);

    // Glow. A body fill, its rim, and the light round it.
    if (Mode == 0) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        if (m > 0.01) {
            float edge = edgeRing(1.6 * pxScale);
            float rim = smoothstep(0.0, 0.25, edge) * outline;
            float inner = (1.0 - smoothstep(0.5, 0.95, g.a)) * 0.35;
            float a = clamp(fill * (0.7 + inner) + rim, 0.0, 1.0) * m;
            fragColor = vec4(pc * a, a);
        } else {
            float s = outerGlow(g) * intensity;
            fragColor = vec4(pc * s, clamp(s * 0.25, 0.0, 1.0));
        }
        return;
    }

    // Blur. The frame itself, softened by the scene copy, tinted by the body.
    if (Mode == 1) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        if (m > 0.01) {
            vec3 sc = textureLod(Scene, uv, 0.0).rgb;
            vec3 col = mix(sc, sc * pc * 1.25 + pc * 0.10, tintAmount);
            col = col * 0.9 + 0.06;
            float edge = edgeRing(1.6 * pxScale);
            float rim = smoothstep(0.0, 0.25, edge) * outline;
            float inner = 1.0 - smoothstep(0.5, 0.95, g.a);
            col += pc * (rim * 0.9 + inner * 0.25);
            fragColor = vec4(col * m, m);
        } else {
            float s = outerGlow(g) * 0.6 * intensity;
            fragColor = vec4(pc * s, clamp(s * 0.2, 0.0, 1.0));
        }
        return;
    }

    // Galaxy. Nebulae and motes inside each body.
    if (Mode == 2) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        if (m > 0.01) {
            vec2 p = vec2(uv.x * aspect, uv.y) * 3.0;
            float t = time * 0.03;
            float n1 = fbm2(p + vec2(t, -t * 0.6));
            float n2 = fbm2(p * 1.9 + vec2(5.2, 1.3) - vec2(t * 0.7, 0.0));
            vec3 deep = vec3(0.02, 0.01, 0.06);
            vec3 nebA = mix(vec3(0.35, 0.10, 0.75), pc, 0.45);
            vec3 nebB = mix(vec3(0.05, 0.55, 0.95), pc, 0.25);
            vec3 col = deep + nebA * pow(smoothstep(0.35, 0.9, n1), 1.6) * 1.2
                            + nebB * pow(smoothstep(0.45, 0.95, n2), 2.0) * 0.9;
            col += vec3(1.0, 0.95, 0.9) * (stars(p + t * 0.5, 22.0, time)
                                        + stars(p * 1.7 + 3.0, 36.0, time * 1.3) * 0.7);
            float edge = edgeRing(1.6 * pxScale);
            float rim = smoothstep(0.0, 0.25, edge) * outline;
            float inner = 1.0 - smoothstep(0.5, 0.95, g.a);
            col += pc * (rim * 1.1 + inner * 0.45);
            fragColor = vec4(col * m, m);
        } else {
            float s = outerGlow(g) * 0.7 * intensity;
            fragColor = vec4(pc * s, clamp(s * 0.2, 0.0, 1.0));
        }
        return;
    }

    // Prism. Two accents drifting as light fields.
    if (Mode == 3) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        vec3 tint = mix(accent, accent2, 0.5 + 0.5 * sin(time * 0.8));
        if (m > 0.01) {
            vec2 p = vec2((uv.x - 0.5) * aspect, uv.y - 0.5);
            vec3 col = tint * 0.06 + prismField(p, time) * 1.3;
            float edge = edgeRing(1.6 * pxScale);
            float rim = smoothstep(0.0, 0.25, edge) * outline;
            float inner = 1.0 - smoothstep(0.5, 0.95, g.a);
            col += pc * rim * 1.1 + tint * inner * 0.35;
            fragColor = vec4(col * m, m);
        } else {
            float s = outerGlow(g) * 0.7 * intensity * (0.75 + 0.25 * sin(time * 1.4));
            fragColor = vec4(tint * s, clamp(s * 0.2, 0.0, 1.0));
        }
        return;
    }

    // Neon. A glass tube on the outline, with a hum that ticks.
    if (Mode == 4) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        float hum = 0.92 + 0.08 * sin(time * 40.0) * step(0.96, hash11(floor(time * 5.0)));
        if (m > 0.01) {
            float edge = edgeRing(2.4 * pxScale);
            float tube = smoothstep(0.05, 0.4, edge);
            vec3 col = pc * 0.05 + pc * tube * 1.4 * hum + vec3(1.0) * pow(tube, 3.0) * 0.5 * hum;
            float a = max(0.25, tube);
            fragColor = vec4(col * m * a, m * a);
        } else {
            float s = outerGlow(g) * 0.9 * intensity * hum;
            fragColor = vec4(pc * s, clamp(s * 0.25, 0.0, 1.0));
        }
        return;
    }

    // Scan. Bands and a travelling sweep, with a colour fringe on both edges.
    if (Mode == 5) {
        if (m < 0.01 && g.a < 0.002) discard;
        vec3 pc = playerColor(m, g);
        if (m > 0.01) {
            float lines = 0.55 + 0.45 * step(0.5, fract(gl_FragCoord.y / (3.0 * max(pxScale, 0.5))));
            float sweep = exp(-pow((fract(uv.y * 2.0 + time * 0.45) - 0.5) * 12.0, 2.0));
            float edge = smoothstep(0.0, 0.3, edgeRing(1.6 * pxScale)) * outline;
            float er = cov(uv + vec2(texel.x * 2.0 * pxScale, 0.0));
            float eb = cov(uv - vec2(texel.x * 2.0 * pxScale, 0.0));
            vec3 fringe = vec3(1.0 - er, 0.0, 1.0 - eb) * 0.8;
            vec3 col = pc * (0.25 * lines + sweep * 0.9) + pc * edge + fringe;
            float a = clamp(0.35 * lines + sweep * 0.5 + edge, 0.0, 1.0);
            fragColor = vec4(col * m * a, m * a);
        } else {
            float s = outerGlow(g) * 0.55 * intensity;
            fragColor = vec4(pc * s, clamp(s * 0.2, 0.0, 1.0));
        }
        return;
    }

    // Liquid Glass. The scene bent and split by a rounded slab of glass.
    if (Mode == 6) {
        if (m < 0.01) discard;
        vec3 pc = playerColor(m, g);
        vec2 e = glowTexel * 1.5;
        vec2 grad = vec2(textureLod(Glow, uv + vec2(e.x, 0.0), 0.0).a
                       - textureLod(Glow, uv - vec2(e.x, 0.0), 0.0).a,
                         textureLod(Glow, uv + vec2(0.0, e.y), 0.0).a
                       - textureLod(Glow, uv - vec2(0.0, e.y), 0.0).a);
        float gl = length(grad);
        vec2 nOut = gl > 1e-5 ? -grad / gl : vec2(0.0);
        float R = thickness * pxScale;
        // `near` is a reserved word in some GLSL dialects, so the edge share is
        // spelled out here rather than copied name for name.
        float nearEdge = (edgeRing(R * 0.33) + edgeRing(R * 0.66) + edgeRing(R)) / 3.0;
        float bevel = clamp(nearEdge * 2.2, 0.0, 1.0);
        float slope = bevel * bevel * bevel;
        float d = 1.0 - bevel;
        vec2 offs = -nOut * slope * refraction * texel * 20.0 * pxScale;
        float disp = dispersion * slope;
        vec3 col;
        col.r = textureLod(Scene, uv + offs * (1.0 + disp), 0.0).r;
        col.g = textureLod(Scene, uv + offs, 0.0).g;
        col.b = textureLod(Scene, uv + offs * (1.0 - disp), 0.0).b;
        col = mix(col, col * (0.75 + 0.5 * pc) + pc * 0.04, tintAmount);
        col = mix(vec3(luma(col)), col, 1.08) * 1.03;
        vec2 L = normalize(vec2(-0.6, 0.8));
        float facing = max(dot(nOut, L), 0.0);
        float away = max(dot(nOut, -L), 0.0);
        float rim = smoothstep(0.6, 0.0, d);
        col += vec3(1.0) * pow(rim, 2.0) * (0.04 + 0.96 * facing) * 0.7 * intensity;
        col += vec3(1.0) * pow(rim, 3.0) * away * 0.35 * intensity;
        float hair = smoothstep(0.05, 0.4, edgeRing(1.3 * pxScale));
        col += mix(vec3(1.0), pc, 0.4) * hair * (0.3 + 0.5 * facing) * outline * intensity;
        col *= 1.0 - 0.16 * smoothstep(0.6, 0.0, d) * away;
        col += 0.025 * intensity;
        fragColor = vec4(col * m, m);
        return;
    }

    // Outline. A crisp contour at a fixed distance round the silhouette.
    if (Mode == 7) {
        if (m < 0.01 && g.a < 0.05) discard;
        vec3 pc = playerColor(m, g);
        const float T = 0.159;
        float fw = max(fwidth(g.a), 1e-4);
        float outer = smoothstep(T - fw, T + fw, g.a);
        float band = outer * (1.0 - m);
        float hair = 0.0;
        if (thinInner > 0.5) {
            hair = 1.0 - smoothstep(0.0, 1.6 * fw, abs(g.a - 0.5));
            hair *= m;
        }
        float a = clamp(band + hair * 0.9, 0.0, 1.0) * clamp(intensity, 0.0, 1.0);
        vec3 col = pc * a;
        float fa = m * fill;
        col += pc * fa * (1.0 - a);
        a = a + fa * (1.0 - a);
        fragColor = vec4(col, a);
        return;
    }

    // Contour. A topographic map of the blurred silhouette.
    if (Mode == 8) {
        if (m < 0.01 && g.a < 0.02) discard;
        vec3 pc = playerColor(m, g);
        float n = max(lineCount, 2.0);
        float drift = fract(time * 0.12);
        float lines = 0.0;
        if (m > 0.5) {
            float v = (g.a - 0.5) * 2.0 * n - drift;
            float fv = max(fwidth(v), 1e-4);
            float l = 1.0 - smoothstep(0.35 * fv, 1.3 * fv, 0.5 - abs(fract(v) - 0.5));
            lines = l * 0.65;
        } else {
            float v = g.a / 0.5 * 2.0 + drift;
            float fv = max(fwidth(v), 1e-4);
            float l = 1.0 - smoothstep(0.35 * fv, 1.3 * fv, 0.5 - abs(fract(v) - 0.5));
            lines = l * smoothstep(0.18, 0.45, g.a) * 0.45;
        }
        float edge = iso(g.a, 0.5, 1.6) * outline;
        float a = clamp(max(lines, edge) * intensity + m * fill, 0.0, 1.0);
        vec3 col = mix(pc, vec3(1.0), edge * 0.35) * a;
        fragColor = vec4(col, a);
        return;
    }

    // Cel. A flat illustration: toon tones from the silhouette's own normals.
    if (Mode == 9) {
        if (m < 0.01 && g.a < 0.05) discard;
        vec3 pc = playerColor(m, g);
        const float T = 0.159;
        float fw = max(fwidth(g.a), 1e-4);
        float ink = smoothstep(T - fw, T + fw, g.a) * (1.0 - m) * outline;
        vec2 e = glowTexel * 1.5;
        vec2 grad = vec2(textureLod(Glow, uv + vec2(e.x, 0.0), 0.0).a
                       - textureLod(Glow, uv - vec2(e.x, 0.0), 0.0).a,
                         textureLod(Glow, uv + vec2(0.0, e.y), 0.0).a
                       - textureLod(Glow, uv - vec2(0.0, e.y), 0.0).a);
        vec2 nOut = length(grad) > 1e-5 ? -normalize(grad) : vec2(0.0);
        float depth = clamp((g.a - 0.5) * 2.0, 0.0, 1.0);
        float lit = dot(nOut, normalize(vec2(-0.6, 0.8))) * (1.0 - depth);
        float tone = lit > 0.25 ? 1.18 : (lit < -0.25 ? 0.62 : 0.88);
        float body = m * clamp(fill, 0.0, 1.0);
        vec3 bodyCol = pc * tone;
        vec3 inkCol = pc * 0.18;
        float a = clamp(body + ink, 0.0, 1.0) * clamp(intensity, 0.0, 1.0);
        vec3 col = (bodyCol * body + inkCol * ink) * clamp(intensity, 0.0, 1.0);
        fragColor = vec4(col, a);
        return;
    }

    // Pulse. The outline breathes, and every beat sends a ring outward.
    if (m < 0.01 && g.a < 0.01) discard;
    vec3 pc = playerColor(m, g);
    float T = mix(0.24, 0.17, clamp(bass, 0.0, 1.0));
    float fw = max(fwidth(g.a), 1e-4);
    float band = smoothstep(T - fw, T + fw, g.a) * (1.0 - m);
    float soft = pow(clamp(g.a * 2.0, 0.0, 1.0), 1.4) * (1.0 - m)
               * (0.08 + 0.22 * bass + 0.12 * flash);
    float rings = 0.0;
    for (int i = 0; i < 4; i++) {
        float age = beatAge[i];
        if (age < 0.0 || age > 1.6) continue;
        float lvl = 0.30 * exp(-age * 2.2);
        float w = max(fw * 1.5, 0.01);
        rings += exp(-pow((g.a - lvl) / w, 2.0)) * exp(-age * 1.8)
               * smoothstep(0.0, 0.25, age) * (1.0 - m);
    }
    float a = clamp(band * (0.85 + 0.15 * flash) + soft + rings * 0.5 + m * fill, 0.0, 1.0);
    a *= clamp(intensity, 0.0, 1.5);
    vec3 col = mix(pc, vec3(1.0), 0.1 * flash) * a;
    fragColor = vec4(col, min(a, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
    #[test]
    fn every_stage_is_glsl_150_for_shaderloader() {
        for stage in [
            MASK_VERTEX,
            MASK_FRAGMENT,
            POST_VERTEX,
            DOWN_FRAGMENT,
            BLUR_FRAGMENT,
            COMPOSITE_FRAGMENT,
        ] {
            assert!(stage.starts_with("#version 150\n"), "GLSL 150 profile");
            assert!(
                !stage.contains("#version 330"),
                "not the 330 source verbatim"
            );
        }
        assert!(MASK_VERTEX.contains("in vec3 Position"));
        assert!(POST_VERTEX.contains("gl_VertexID"));
    }

    #[test]
    fn the_composite_declares_every_uniform_the_pass_sets() {
        for uniform in [
            "uniform sampler2D Mask;",
            "uniform sampler2D Glow;",
            "uniform sampler2D Scene;",
            "uniform vec2 texel;",
            "uniform vec2 glowTexel;",
            "uniform float pxScale;",
            "uniform float time;",
            "uniform float outline;",
            "uniform float intensity;",
            "uniform float fill;",
            "uniform float tintAmount;",
            "uniform float aspect;",
            "uniform vec3 accent;",
            "uniform vec3 accent2;",
            "uniform float thinInner;",
            "uniform float lineCount;",
            "uniform float refraction;",
            "uniform float dispersion;",
            "uniform float thickness;",
            "uniform float bass;",
            "uniform float beatAge[4];",
            "uniform float flash;",
            "uniform int Mode;",
        ] {
            assert!(
                COMPOSITE_FRAGMENT.contains(uniform),
                "composite is missing {uniform}"
            );
        }
    }

     
     
    #[test]
    fn every_look_has_a_branch() {
        for mode in 0..=9 {
            assert!(
                COMPOSITE_FRAGMENT.contains(&format!("if (Mode == {mode})")),
                "look {mode} has no branch"
            );
        }
        assert!(
            COMPOSITE_FRAGMENT.contains("beatAge[i]"),
            "Pulse reads the beat"
        );
    }
}
