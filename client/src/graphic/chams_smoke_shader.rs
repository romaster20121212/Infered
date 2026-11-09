 
pub const VERTEX: &str = r#"#version 150
in vec3 Position;
uniform mat4 View;
uniform mat4 Projection;
uniform vec3 Center;
uniform float Radius;
uniform int Mode; // ADD TO: 0 smoke, 1 orb, 2 actual player geometry
out vec3 worldPoint;
out vec2 uv;
out float viewDepth;
void main() {
    uv = Position.xy;
    vec3 right = vec3(View[0][0], View[1][0], View[2][0]);
    vec3 up = vec3(View[0][1], View[1][1], View[2][1]);
    worldPoint = Mode == 2 ? Position : Center + Radius * (right * uv.x + up * uv.y);
    vec4 eye = View * vec4(worldPoint, 1.0);
    viewDepth = -eye.z;
    gl_Position = Projection * eye;
}
"#;

pub const FRAGMENT: &str = r#"#version 150
uniform sampler2D SceneDepth;
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
in vec3 worldPoint;
in vec2 uv;
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
float density(vec3 p) {
    float edge = 1.0 - smoothstep(0.52, 1.0, length(p));
    vec3 q = p*2.4 + vec3(Seed, Seed*0.73, Seed*0.31) + vec3(0.13,-0.21,0.08)*Time;
    float n = 0.57*smokeNoise(q) + 0.28*smokeNoise(q*2.03) + 0.15*smokeNoise(q*4.11);
    return edge * smoothstep(0.22, 0.72, n);
}
void main() {
    if (viewDepth <= 0.05) discard;
    if (Mode == 2) {
        // Trail material: flowing mix of dark black (14,12,13) and neon red / Tint
        // with Trail's 10 rad/s shimmer ripple and emissive neon glow on bright veins.
        vec3 darkBlack = vec3(14.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0);
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
