 
pub const VERTEX: &str = r#"#version 150
in vec3 Position;
// Held items carry the atlas coordinate of the quad they came from; every other
// pass leaves the attribute off, and it then reads as a constant.
in vec2 ItemUv;
uniform mat4 View;
uniform mat4 Projection;
uniform vec3 Center;
uniform float Radius;
uniform int Mode; // 0 smoke, 1 orb, 2 player geometry, 3 blast, 8 hologram
out vec3 worldPoint;
out vec2 uv;
out vec2 itemUv;
out float viewDepth;
void main() {
    uv = Position.xy;
    itemUv = ItemUv;
    vec3 right = vec3(View[0][0], View[1][0], View[2][0]);
    vec3 up = vec3(View[0][1], View[1][1], View[2][1]);
    // Mode 2/8/9 use the exact player triangles, already camera-relative:
    // chams and both hologram layers all draw model geometry; mode 10 is the
    // skeleton's bone meshes, which are built the same way.
    bool modelGeometry = Mode == 2 || Mode == 8 || Mode == 9 || Mode == 10;
    worldPoint = modelGeometry ? Position : Center + Radius * (right * uv.x + up * uv.y);
    vec4 eye = View * vec4(worldPoint, 1.0);
    viewDepth = -eye.z;
    gl_Position = Projection * eye;
}
"#;

pub const FRAGMENT: &str = r#"#version 150
uniform sampler2D SceneDepth;
uniform sampler2D Video; // chams video material: the current frame, screen-projected
uniform sampler2D ItemTex; // chams item material: the stitched item atlas
uniform int ItemCutout; // 1 = keep only fragments the item's sprite covers
uniform mat4 InverseProjection;
uniform vec4 Viewport;
uniform float Radius;
uniform float Time;
uniform float Seed;
uniform float Alpha;
uniform float Light;
uniform vec3 Tint;
uniform int Mode;
uniform int OrbCount;
uniform vec4 OrbPositions[24]; // camera-relative xyz; local light radius in w
uniform vec4 OrbColors[24]; // rgb; shared-pulse/lifetime/flicker intensity in w
uniform vec3 CameraForward;
uniform int Style; // chams look: 0 neon trail, 1 solid, 2 scanline, 3 ember, 4 video
in vec3 worldPoint;
in vec2 uv;
in vec2 itemUv;
in float viewDepth;
out vec4 fragColor;
float hash(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}
float smokeNoise(vec3 p) {
    vec3 i = floor(p), f = fract(p);
    f = f*f*(3.0-2.0*f);
    return mix(mix(mix(hash(i),hash(i+vec3(1,0,0)),f.x),
                   mix(hash(i+vec3(0,1,0)),hash(i+vec3(1,1,0)),f.x),f.y),
               mix(mix(hash(i+vec3(0,0,1)),hash(i+vec3(1,0,1)),f.x),
                   mix(hash(i+vec3(0,1,1)),hash(i+vec3(1,1,1)),f.x),f.y),f.z);
}
// Rebuilds a surface normal packed into the material's uv slot, which is what
// lets the bone pass share the chams vertex layout and program.
vec3 octNormal(vec2 e) {
    vec3 n = vec3(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
    if (n.z < 0.0) n.xy = (1.0 - abs(n.yx)) * vec2(n.x < 0.0 ? -1.0 : 1.0, n.y < 0.0 ? -1.0 : 1.0);
    return normalize(n);
}
float density(vec3 p) {
    float edge = 1.0 - smoothstep(0.52, 1.0, length(p));
    vec3 q = p*2.4 + vec3(Seed, Seed*0.73, Seed*0.31) + vec3(0.13,-0.21,0.08)*Time;
    float n = 0.57*smokeNoise(q) + 0.28*smokeNoise(q*2.03) + 0.15*smokeNoise(q*4.11);
    return edge * smoothstep(0.22, 0.72, n);
}
void main() {
    if (viewDepth <= 0.05) discard;
    if (Mode == 2) {
        // Held items: the model's quad can be a full square (a sword's flat
        // model is one), so the material is cut to the sprite it samples. That
        // is what makes an item chams the shape of the item instead of a slab.
        // A vertex with no coordinate from the game is left alone, so a model
        // that draws no sprite still shows its own geometry. The threshold is
        // low on purpose: only the sprite's empty space is dropped, never a soft
        // edge, so a model mapped to another texture keeps its own shape.
        if (ItemCutout != 0 && itemUv.x >= -0.5 && texture(ItemTex, itemUv).a < 0.1) discard;
        vec3 darkBlack = vec3(14.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0);
        if (Style == 4) {
            // Video: the frame the clip's clock is on, sampled with screen
            // coordinates. Each body is a window onto the current frame, so the
            // material does not need the model's own UVs (the geometry carries
            // none) and never stretches with the body's pose.
            vec2 screenUv = (gl_FragCoord.xy - Viewport.xy) / Viewport.zw;
            vec3 frame = texture(Video, vec2(screenUv.x, 1.0 - screenUv.y)).rgb;
            vec3 video = frame * (0.55 + 0.45 * Light);
            fragColor = vec4(video * Alpha, Alpha);
            return;
        }
        if (Style == 1) {
            // Solid: flat Tint with a slow brightness breathe.
            float breathe = 0.92 + 0.08 * sin(Time * 4.0 + worldPoint.y * 3.0);
            vec3 solid = Tint * (0.55 + 0.45 * Light) * breathe;
            fragColor = vec4(solid * Alpha, Alpha);
            return;
        }
        if (Style == 2) {
            // Scanline: dark bands and Tint bands sliding up the model.
            float band = 0.5 + 0.5 * sin(worldPoint.y * 38.0 - Time * 6.0);
            vec3 lines = mix(darkBlack, Tint, smoothstep(0.3, 0.7, band));
            vec3 scan = lines * (0.6 + 0.4 * Light);
            fragColor = vec4(scan * Alpha, Alpha);
            return;
        }
        if (Style == 3) {
            // Ember: noise-driven hot spots of Tint over dark black, rising over time.
            float n = smokeNoise(worldPoint * 3.0 + vec3(0.0, -Time * 1.2, 0.0));
            float hot = smoothstep(0.55, 0.9, n);
            vec3 ember = mix(darkBlack, Tint, hot) + Tint * hot * 0.6 * Light;
            fragColor = vec4(ember * Alpha, Alpha);
            return;
        }
        // Style 0, Neon Trail (the default): flowing mix of dark black (14,12,13) and neon red / Tint
        // with Trail's 10 rad/s shimmer ripple and emissive neon glow on bright veins.
        vec3 flowCoord = worldPoint * 2.6 + vec3(0.0, -Time * 1.8, Time * 0.45);
        float n = 0.62 * smokeNoise(flowCoord) + 0.38 * smokeNoise(flowCoord * 2.15 + vec3(3.7, -Time * 2.4, 1.9));
        float wave = 0.5 + 0.5 * sin(worldPoint.y * 5.5 - Time * 10.0 + n * 4.5);
        float mixFactor = clamp(smoothstep(0.20, 0.80, 0.65 * n + 0.35 * wave), 0.0, 1.0);
        float shimmer = 1.0 + 0.18 * sin(Time * 10.0 - worldPoint.y * 4.0);
        vec3 baseColor = mix(darkBlack, Tint, mixFactor);
        float neonGlow = mixFactor * mixFactor * 0.45 * Light;
        vec3 finalColor = (baseColor * (0.35 + 0.65 * Light * shimmer) + Tint * neonGlow) * Alpha;
        fragColor = vec4(finalColor, Alpha);
        return;
    }
    if (Mode == 9) {
        // Hologram core: the body the skin would otherwise show, replaced by a
        // dark tinted solid. Alpha is exactly 1 so it really covers the model
        // drawn underneath — that is what makes this chams and not a tint over
        // the skin — while the scan structure keeps it from reading as a slab.
        float scan = 0.5 + 0.5 * sin(worldPoint.y * 46.0 - Time * 5.5);
        vec3 core = Tint * (0.16 + 0.10 * scan) * (0.6 + 0.4 * Light);
        fragColor = vec4(core, 1.0);
        return;
    }
    if (Mode == 8) {
        // Hologram: the player's own triangles as a projected volume. The pass
        // writes no depth and draws both faces, so the far side of the body
        // blends under the near side and the figure reads as something you can
        // see through, not a skin. Travelling scanlines, a slow vertical sweep
        // and a little grain keep it from looking like flat paint.
        float scan = 0.55 + 0.45 * sin(worldPoint.y * 46.0 - Time * 5.5);
        float sweepY = fract(Time * 0.11);
        float sweep = 1.0 - abs(fract(worldPoint.y * 0.5) - sweepY) * 2.0;
        sweep = smoothstep(0.86, 1.0, sweep);
        float grain = smokeNoise(worldPoint * 7.0 + vec3(0.0, -Time * 0.7, Time * 0.2));
        // A slight lift towards the head, so the legs do not read as a brighter
        // puddle under the figure.
        float height = 0.75 + 0.25 * clamp(worldPoint.y + 0.5, 0.0, 1.0);
        float density = (0.55 + 0.45 * scan) * height * (0.85 + 0.15 * grain);
        vec3 color = Tint * (0.6 + 0.4 * Light) * (0.75 + 0.5 * scan);
        color += Tint * sweep * 0.8 * Light;
        float alpha = clamp(Alpha * density + Alpha * sweep * 0.45, 0.0, 1.0);
        fragColor = vec4(color * alpha, alpha);
        return;
    }
    if (Mode == 10) {
        // Skeleton bones: a shaded capsule, not a painted line. A soft key light
        // gives the surface its form and a camera-facing rim keeps the far edge
        // of a bone readable against both a bright sky and dark terrain.
        vec3 n = octNormal(itemUv);
        vec3 viewDir = viewDepth > 1e-4 ? normalize(worldPoint) : vec3(0.0, 0.0, -1.0);
        float key = clamp(dot(n, normalize(vec3(0.42, 0.78, 0.31))) * 0.5 + 0.5, 0.0, 1.0);
        float rim = pow(1.0 - clamp(abs(dot(n, viewDir)), 0.0, 1.0), 2.0);
        vec3 bone = Tint * (0.40 + 0.60 * key) + Tint * rim * 0.55 * Light;
        fragColor = vec4(bone * Alpha, Alpha);
        return;
    }
    float radial = dot(uv, uv);
    if (radial >= 1.0) discard;
    vec2 screenUv = (gl_FragCoord.xy - Viewport.xy) / Viewport.zw;
    float depth = texture(SceneDepth, screenUv).r;
    vec4 scene = InverseProjection * vec4(screenUv*2.0-1.0, depth*2.0-1.0, 1.0);
    float sceneDepth = -scene.z / scene.w;
    if (Mode == 3) {
        // Explosion blast: a torn fireball, not smoke density. A hot white core,
        // an ember body whose rim churns with animated noise, and a brief shock ring.
        float r = sqrt(radial);
        float angle = atan(uv.y, uv.x);
        float tear = smokeNoise(vec3(cos(angle), sin(angle), Seed) * 1.7 + vec3(0.0, 0.0, Time * 2.6));
        float boundary = 0.84 - 0.28 * tear;
        float body = 1.0 - smoothstep(boundary - 0.24, boundary, r);
        float core = exp(-radial * 7.0);
        float shock = exp(-pow((r - 0.93) * 11.0, 2.0));
        float flicker = 0.88 + 0.12 * sin(Time * 17.0 + Seed * 6.2831);
        float occlusion = smoothstep(0.0, 0.08, sceneDepth - viewDepth);
        float a = clamp((body * 0.9 + shock * 0.35 + core * 0.6) * Alpha * occlusion * flicker, 0.0, 1.0);
        if (a < 0.002) discard;
        vec3 ember = Tint * mix(0.45, 1.0, body);
        vec3 color = mix(ember, vec3(1.0, 0.95, 0.8), core) * Light * (0.75 + 0.5 * core);
        fragColor = vec4(color * a, a); // premultiplied alpha; ONE, ONE_MINUS_SRC_ALPHA
        return;
    }
    if (Mode == 1) {
        float occlusion = smoothstep(0.0, 0.08, sceneDepth-viewDepth);
        float sphere = sqrt(1.0-radial);
        float a = Alpha * occlusion * pow(sphere, 2.0);
        vec3 glow = Tint * (0.65 + 0.35*sphere) * Light;
        fragColor = vec4(glow*a,a);
        return;
    }
    float span = sqrt(1.0-radial);
    float total = 0.0, illumination = 0.0;
    vec3 orbLight = vec3(0.0);
    // Four density slices: a cheap volume-looking impostor, not physical transport.
    for (int i=0; i<4; ++i) {
        float z = mix(-span,span,(float(i)+0.5)*0.25);
        vec3 p = vec3(uv,z);
        float d = density(p);
        float soft = smoothstep(0.0,max(0.12,Radius*0.28),sceneDepth-(viewDepth+z*Radius));
        d *= soft;
        vec3 lightDir = normalize(vec3(-0.45,0.8,-0.4));
        float shadow = density(p+lightDir*0.22) + density(p+lightDir*0.45);
        total += d;
        illumination += d * (0.32 + 0.68*exp(-shadow*1.6));
        // Artistic local light falloff at the four smoke samples; no shadows/lightmap changes.
        vec3 samplePoint = worldPoint + CameraForward * z * Radius;
        for (int j=0; j<OrbCount; ++j) {
            float falloff = max(0.0, 1.0-distance(samplePoint,OrbPositions[j].xyz)/OrbPositions[j].w);
            orbLight += d * OrbColors[j].rgb * OrbColors[j].w * falloff * falloff;
        }
    }
    float a = (1.0-exp(-total*span*0.75))*Alpha;
    a *= smoothstep(0.08,0.65,viewDepth-Radius*0.3);
    if (a < 0.002) discard;
    float shade = illumination / max(total,0.0001);
    vec3 color = Tint * Light * shade + orbLight/max(total,0.0001)*0.45;
    fragColor = vec4(color*a,a); // premultiplied alpha; ONE, ONE_MINUS_SRC_ALPHA
}
"#;
