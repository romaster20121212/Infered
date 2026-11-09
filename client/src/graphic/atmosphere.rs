 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use super::smoke::GlGuard;
use crate::gl;
use crate::mapping::MappedObject;
use crate::state::{mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{JClass, JValue};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

const VERTEX: &str = r#"#version 150
in vec3 Position;
void main() {
    gl_Position = vec4(Position.xy, 1.0, 1.0);
}
"#;

 
 
 
 
 
 
 
 
const FRAGMENT: &str = r#"#version 150
uniform float u_time;
uniform float u_mode;
uniform float u_density;
uniform float u_star_size;
uniform float u_twinkle;
uniform float u_glow;
uniform float u_hole;
uniform float u_spin;
uniform float u_noise_scale;
uniform float u_flow;
uniform float u_brightness;
uniform vec3 u_nebula_color;
uniform vec3 u_lattice_color;
uniform vec3 u_frost_color;
uniform vec3 u_aurora_color;
uniform vec3 u_ember_color;
uniform vec3 u_rain_color;
uniform vec2 u_resolution;
uniform vec3 u_forward;
uniform vec3 u_right;
uniform vec3 u_up;
uniform float u_fov;
uniform vec3 u_origin;
uniform float u_parallax;
uniform vec2 u_hole_centre;
uniform sampler2D u_custom;
uniform sampler2D u_lyrics;
uniform float u_lyrics_heading;
uniform float u_lyrics_alpha;
uniform float u_lyrics_progress;
uniform vec3 u_lyrics_text;
uniform vec3 u_lyrics_highlight;
out vec4 fragColor;

float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Four octaves of value noise — the nebula band and the disk's grain.
float fbm(vec2 p) {
    float sum = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 4; i++) {
        sum += vnoise(p) * amp;
        p *= 2.03;
        amp *= 0.5;
    }
    return sum;
}

// --- Noise studies -------------------------------------------------------
//
// The three extra scenes visualise a different field each. Their palettes are
// rebuilt from a per-scene accent (the client's brand red by default): a
// near-black body tinted toward the accent, the accent itself for the bright
// faces, and a pale crest mixed toward white. Picking another colour therefore
// recolours the whole study, not just its seams.
const vec3 NOISE_BLACK = vec3(0.030, 0.012, 0.014);

vec3 noiseBody(vec3 accent) {
    return accent * 0.40;
}

vec3 noiseEmber(vec3 accent) {
    return mix(accent, vec3(1.0), 0.55);
}

// Unit gradient for gradient noise, picked from the cell hash.
vec2 gradient(vec2 cell) {
    float angle = hash21(cell) * 6.2831853;
    return vec2(cos(angle), sin(angle));
}

// Classic gradient (Perlin) noise with the quintic fade: zero at every lattice
// point, smooth in between, with none of the blocky grid value noise shows.
float perlin(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = dot(gradient(i), f);
    float b = dot(gradient(i + vec2(1.0, 0.0)), f - vec2(1.0, 0.0));
    float c = dot(gradient(i + vec2(0.0, 1.0)), f - vec2(0.0, 1.0));
    float d = dot(gradient(i + vec2(1.0, 1.0)), f - vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Five octaves, each rotated and offset so the lattices never line up into a
// visible grid. Normalised back to roughly -0.6..0.6.
float perlin_fbm(vec2 p) {
    float sum = 0.0;
    float amplitude = 0.55;
    float total = 0.0;
    mat2 turn = mat2(0.80, 0.60, -0.60, 0.80);
    for (int i = 0; i < 5; ++i) {
        sum += perlin(p) * amplitude;
        total += amplitude;
        p = turn * p * 2.03 + vec2(7.3, 3.1);
        amplitude *= 0.5;
    }
    return sum / max(total, 0.0001);
}

// Worley (cellular) noise: the nearest feature distance and the second nearest.
// Their gap is zero on a cell border and rises toward the cell centre.
vec2 worley(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float first = 8.0;
    float second = 8.0;
    for (int y = -1; y <= 1; ++y) {
        for (int x = -1; x <= 1; ++x) {
            vec2 g = vec2(float(x), float(y));
            vec2 offset = vec2(hash21(i + g), hash21(i + g + vec2(19.19, 7.07)));
            float d = length(g + offset - f);
            if (d < first) {
                second = first;
                first = d;
            } else if (d < second) {
                second = d;
            }
        }
    }
    return vec2(first, second);
}

// Ridged multifractal: gradient noise folded at zero, so its ridges become thin
// filaments instead of soft hills.
float ridged(vec2 p) {
    float sum = 0.0;
    float amplitude = 0.65;
    float total = 0.0;
    for (int i = 0; i < 4; ++i) {
        float ridge = 1.0 - abs(perlin(p));
        ridge *= ridge;
        sum += ridge * amplitude;
        total += amplitude;
        p = p * 2.07 + vec2(11.7, 5.3);
        amplitude *= 0.55;
    }
    return clamp(sum / max(total, 0.0001), 0.0, 1.0);
}

// `1` inside `inner`, fading to `0` at `outer`. Written out because GLSL leaves
// smoothstep undefined for reversed edges, and these bands need both orders.
float falloff(float x, float inner, float outer) {
    return 1.0 - smoothstep(inner, outer, x);
}

// One layer of stars on a jittered grid. Each star gets its own size, colour
// temperature and twinkle phase from the cell hash, so a field never pulses in
// unison. `seed` decorrelates the layers.
float stars(vec2 p, float cells, float seed, float size, float twinkle) {
    vec2 grid = p * cells + seed;
    vec2 cell = floor(grid);
    vec2 local = fract(grid) - 0.5;
    float rnd = hash21(cell + seed * 7.13);
    // Only a fraction of cells carry a star; the rest stay empty.
    if (rnd < 0.62) {
        return 0.0;
    }
    vec2 offset = vec2(hash21(cell + 3.7), hash21(cell + 9.1)) - 0.5;
    float dist = length(local - offset * 0.62);
    float radius = size * (0.018 + rnd * 0.030);
    float core = falloff(dist, 0.0, radius);
    float halo = u_glow * falloff(dist, 0.0, radius * 6.5) * 0.34;
    float flicker = 0.55 + 0.45 * sin(u_time * twinkle * (0.6 + rnd * 2.4) + rnd * 41.0);
    return (core + halo) * flicker;
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    vec2 ndc = uv * 2.0 - 1.0;
    float aspect = u_resolution.x / max(u_resolution.y, 1.0);
    float tanHalf = tan(radians(clamp(u_fov, 10.0, 170.0)) * 0.5);

    // The world direction this pixel looks at, from the camera's own basis.
    // Everything below is a function of `dir` (and the player's position), so
    // the sky is fixed in the world and moves only when the player does.
    vec3 dir = normalize(u_forward
        + u_right * (ndc.x * tanHalf * aspect)
        + u_up * (ndc.y * tanHalf));

    // Sky chart: a stereographic map of the dome, the zenith at the origin and
    // `2·tan(θ/2)` from it — roughly one unit per radian overhead, which is the
    // scale the star field was designed at. It is conformal, so stars stay
    // round at every elevation, and the player's position slides the whole
    // chart along with them.
    vec2 p = 2.0 * dir.xz / (1.0 + max(dir.y, -0.90))
          + u_origin.xz * u_parallax;
    float hole = u_hole;
    float upness = clamp(dir.y, 0.0, 1.0);

    // Night gradient: near-black blue overhead deepening to violet at the
    // horizon, so the scene reads as sky rather than a flat backdrop.
    vec3 sky = mix(vec3(0.055, 0.030, 0.105), vec3(0.010, 0.014, 0.040), pow(upness, 0.8));

    float starField = 0.0;
    int mode = int(u_mode + 0.5);
    // Shared coordinates for the noise studies. The other scenes never read
    // them, and they cost a handful of arithmetic operations.
    vec3 noiseDeep = mix(NOISE_BLACK * 1.9, NOISE_BLACK, pow(upness, 0.7));
    vec2 noiseP = p * max(u_noise_scale, 0.05);
    float flow = u_time * u_flow;
    if (mode == 2) {
        // A picture of the player's own, sampled as an equirectangular panorama
        // from the same world directions as the procedural skies: the horizon is
        // the middle row of the image and the zenith its top edge. It is
        // anchored like everything else here, so it turns with the player.
        float u = atan(dir.z, dir.x) * 0.15915494 + 0.5;
        float v = 0.5 - asin(clamp(dir.y, -1.0, 1.0)) * 0.31830989;
        sky = texture(u_custom, vec2(u, v)).rgb;
    } else if (mode == 0) {
        // A slow nebula band across the middle of the sky.
        float band = falloff(abs(p.y + 0.10 * sin(p.x * 1.7 + 0.8)), 0.0, 0.55);
        float clouds = fbm(p * 3.4 + vec2(u_time * 0.008, u_time * 0.004));
        sky += vec3(0.11, 0.06, 0.22) * clouds * band * 0.7;

        starField += stars(p, 7.0, 0.0, u_star_size, u_twinkle);
        starField += stars(p, 15.0, 11.0, u_star_size * 0.8, u_twinkle * 1.3) * 0.7;
        starField += stars(p, 27.0, 5.0, u_star_size * 0.6, u_twinkle * 1.7) * 0.45;

        // Cool white with a few warm stars: tint from the same hash as the
        // twinkle, so colour and brightness belong to the same star.
        float warmth = hash21(floor(p * 7.0));
        vec3 starTint = mix(vec3(0.82, 0.90, 1.0), vec3(1.0, 0.90, 0.78), smoothstep(0.55, 1.0, warmth));
        sky += starTint * starField * u_density;
    } else if (mode == 1) {
        vec2 c = p - u_hole_centre;
        float r = length(c);
        float angle = atan(c.y, c.x);

        // Sparse field, dimmed toward the hole so the accretion ring owns the
        // part of the sky it hangs in.
        float away = smoothstep(hole * 1.1, hole * 2.4, r);
        starField += stars(p, 9.0, 3.0, u_star_size * 0.9, u_twinkle * 0.8) * 0.45;
        starField += stars(p, 19.0, 8.0, u_star_size * 0.7, u_twinkle) * 0.30;
        sky += vec3(0.76, 0.84, 1.0) * starField * away * u_density * 0.70;

        // Accretion disk: a thin hot band just outside the horizon, twisted
        // into a spiral that turns with the spin setting. The brightness leans
        // toward the upper half so the ring reads as a tilted disk, not a halo.
        float disk = falloff(r, hole * 1.02, hole * 1.78) * smoothstep(hole * 0.97, hole * 1.06, r);
        float swirl = 0.55 + 0.45 * sin(angle * 5.0 + r * 26.0 - u_time * u_spin * 3.0);
        float grain = 0.75 + 0.25 * fbm(vec2(angle * 2.4, r * 9.0 - u_time * u_spin));
        float tilt = 0.55 + 0.45 * smoothstep(-1.0, 1.0, c.y / max(hole, 0.001));
        vec3 hot = mix(vec3(1.0, 0.30, 0.03), vec3(1.0, 0.88, 0.62), swirl);
        sky += hot * disk * swirl * grain * tilt * 1.65;

        // The photon ring: a thin, bright circle right at the horizon.
        float rim = falloff(r, hole * 0.99, hole * 1.06) * smoothstep(hole * 0.93, hole * 0.99, r);
        sky += vec3(1.0, 0.80, 0.55) * rim * 1.1;

        // Lensed starlight smeared around the hole.
        float lens = falloff(r, hole * 1.45, hole * 2.60) * smoothstep(hole * 1.10, hole * 1.55, r);
        float ringStars = stars(vec2(cos(angle), sin(angle)) * 2.0 + vec2(r * 9.0, 0.0), 12.0, 2.0, u_star_size * 0.8, u_twinkle * 1.4);
        sky += vec3(0.82, 0.86, 1.0) * ringStars * lens * 1.2;

        // The horizon itself: pure black, eating whatever is behind it.
        float disc = falloff(r, hole * 0.94, hole * 0.99);
        sky = mix(sky, vec3(0.0, 0.0, 0.006), disc);

        // A soft violet halo bleeding into the night.
        sky += vec3(0.42, 0.16, 0.36) * exp(-max(r - hole * 1.9, 0.0) * 5.5) * 0.30;
    } else if (mode == 3) {
        // Ice Nebula: gradient (Perlin) noise, domain-warped into fBm. The warp
        // is what folds the octaves through each other instead of leaving soft
        // blobs; the isolines make the field read as noise rather than fog. The
        // ramp runs black → deep accent → accent → pale crest, so the field
        // looks like burning ink against the night rather than a coloured fog.
        vec3 nebula = u_nebula_color;
        vec2 warp = vec2(perlin(noiseP * 0.8 + vec2(4.1, 1.7)),
                         perlin(noiseP * 0.8 + vec2(8.7, 5.3)));
        float field = perlin_fbm(noiseP * 1.7 + warp * 0.9
            + vec2(flow * 0.030, -flow * 0.022));
        float shade = clamp(field * 0.9 + 0.5, 0.0, 1.0);
        sky = mix(noiseDeep, noiseBody(nebula), smoothstep(0.12, 0.50, shade));
        sky = mix(sky, nebula, smoothstep(0.44, 0.74, shade));
        sky = mix(sky, noiseEmber(nebula), smoothstep(0.78, 0.98, shade));
        float iso = abs(fract(shade * 9.0) - 0.5);
        sky += nebula * (1.0 - smoothstep(0.0, 0.055, iso)) * 0.12;
        sky += noiseEmber(nebula) * pow(smoothstep(0.66, 1.0, shade), 2.6)
             * (0.55 + 0.45 * sin(flow * 0.8 + shade * 16.0)) * 0.28;
    } else if (mode == 4) {
        // Crystal Lattice: Worley cells. The gap between the two nearest
        // feature points draws the borders, the nearest distance lights the
        // cores, and the cell hash gives every facet its own tone.
        vec3 lattice = u_lattice_color;
        vec2 drift = noiseP * 3.0 + vec2(flow * 0.040, flow * 0.018);
        vec2 cells = worley(drift);
        float border = smoothstep(0.0, 0.10, cells.y - cells.x);
        float facet = hash21(floor(drift));
        float core = 1.0 - smoothstep(0.0, 0.55, cells.x);
        // Each cell is a glass pane of the chosen colour, with its own tone;
        // only the seams between panes go crest-bright, so the lattice reads as
        // crystal, not webbing.
        sky = mix(noiseDeep, noiseBody(lattice), 0.42 + 0.42 * facet);
        sky += lattice * core * 0.26;
        sky = mix(sky, lattice, (1.0 - border) * 0.62);
        sky += noiseEmber(lattice) * pow(1.0 - border, 4.0) * 0.34;
    } else if (mode == 5) {
        // Frost Veins: ridged multifractal. Folded ridges become filaments of
        // the chosen colour over black, with the crests breathing slowly.
        // The broad folded field gives the body depth; the veins themselves
        // are the zero-crossings of gradient noise, sharpened into thin lines.
        // A plain broad ramp turned most of the sky white instead.
        float body = ridged(noiseP * 1.4 + vec2(flow * 0.020, flow * 0.036));
        float ridgeA = pow(clamp(1.0 - abs(perlin(noiseP * 2.0
            + vec2(flow * 0.028, flow * 0.052))), 0.0, 1.0), 9.0);
        float ridgeB = pow(clamp(1.0 - abs(perlin(noiseP * 4.3
            - vec2(flow * 0.018, flow * 0.034) + vec2(6.7, 2.9))), 0.0, 1.0), 12.0);
        float crest = clamp(ridgeA * 0.78 + ridgeB * 0.35, 0.0, 1.0);
        vec3 frost = u_frost_color;
        sky = mix(noiseDeep, noiseBody(frost), 0.18 + 0.42 * smoothstep(0.35, 0.78, body));
        sky = mix(sky, frost, crest * 0.78);
        sky += noiseEmber(frost) * pow(crest, 2.5) * 0.60;
        sky *= 0.95 + 0.05 * sin(flow * 0.6 + crest * 18.0);
    } else if (mode == 6) {
        // Aurora Veil: vertical curtains of light. Gradient noise stretched
        // along the vertical, so each fold hangs like a curtain and ripples
        // sideways over time. Brightest in the upper part of the veil.
        vec3 aurora = u_aurora_color;
        float ripple = perlin(vec2(noiseP.x * 0.9 + flow * 0.05, noiseP.y * 0.2));
        float curtainX = noiseP.x * 2.2 + ripple * 1.6 + flow * 0.04;
        float curtain = 1.0 - abs(perlin(vec2(curtainX, noiseP.y * 0.35 - flow * 0.03)));
        curtain = pow(clamp(curtain, 0.0, 1.0), 6.0);
        float shimmer = 0.6 + 0.4 * perlin(vec2(noiseP.x * 5.0 - flow * 0.2, noiseP.y * 1.4));
        float height = smoothstep(-0.05, 0.75, p.y) * (1.0 - smoothstep(0.85, 1.05, p.y));
        float veil = curtain * shimmer * height;
        sky = mix(sky, noiseBody(aurora), 0.35 * height);
        sky += aurora * veil * 0.95;
        sky += noiseEmber(aurora) * pow(veil, 2.0) * 0.30;
    } else if (mode == 7) {
        // Ember Drift: sparse glowing embers drifting upward over a dark
        // ground. The field is offset by time, so the embers keep moving
        // instead of only flickering in place.
        vec3 ember = u_ember_color;
        vec2 drift = p + vec2(0.0, flow * 0.25);
        float embers = stars(drift * 1.4, 11.0, 4.0, u_star_size * 1.1, u_twinkle * 0.9);
        embers += stars(drift * 2.3, 21.0, 9.0, u_star_size * 0.8, u_twinkle * 1.1) * 0.6;
        float glowBase = falloff(p.y, -0.6, 0.2);
        sky = mix(sky, noiseDeep, 0.5 * glowBase);
        sky += ember * embers * (0.55 + 0.45 * u_density) * 1.5;
        sky += noiseEmber(ember) * glowBase * 0.10;
    } else if (mode == 8) {
        // Starfall Rain: thin streaks of light falling diagonally over a
        // deep night sky. Time moves the field downward, so the streaks fall.
        vec3 rain = u_rain_color;
        vec2 fall = vec2(p.x + p.y * 0.35, p.y + flow * 0.9);
        float streaks = stars(vec2(fall.x * 2.0, fall.y * 0.35), 13.0, 31.0, u_star_size * 0.7, u_twinkle * 0.5);
        streaks += stars(vec2(fall.x * 3.1, fall.y * 0.5), 23.0, 37.0, u_star_size * 0.5, u_twinkle * 0.5) * 0.5;
        sky = mix(sky, noiseDeep, 0.35);
        sky += rain * streaks * (0.5 + 0.5 * u_density) * 1.3;
    }

    // Vignette plus a whisper of grain: both are lens effects, so they stay in
    // screen space. The grain hides banding in the dark gradient on 8-bit
    // framebuffers.
    float vig = mix(0.72, 1.0, falloff(length(ndc * vec2(aspect * 0.41, 0.5)), 0.25, 1.55));
    sky *= vig;
    sky += (hash21(uv * u_resolution + fract(u_time) * 91.7) - 0.5) * 0.014;

    float azimuth = atan(dir.z, dir.x) - u_lyrics_heading;
    float delta = atan(sin(azimuth), cos(azimuth));
    vec2 lyricUV = vec2(0.5 + delta / 1.35,
                       (0.5 - asin(clamp(dir.y, -1.0, 1.0)) * 0.31830989 - 0.25) / 0.20);
    if (u_lyrics_alpha > 0.0 && lyricUV.x >= 0.0 && lyricUV.x <= 1.0
        && lyricUV.y >= 0.0 && lyricUV.y <= 1.0) {
        float ink = texture(u_lyrics, lyricUV).r * u_lyrics_alpha;
        vec3 lyricColor = mix(u_lyrics_text, u_lyrics_highlight,
                              1.0 - step(u_lyrics_progress, lyricUV.x));
        sky = mix(sky * u_brightness, lyricColor, ink);
    } else {
        sky *= u_brightness;
    }
    fragColor = vec4(sky, 1.0);
}
"#;

 
const QUAD: [f32; 18] = [
    -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, 1.0, 0.0,  
    -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0,
];

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyMode {
     
    StarryNight,
     
    BlackHole,
     
    Custom,
     
    IceNebula,
     
    CrystalLattice,
     
    FrostVeins,
     
    AuroraVeil,
     
    EmberDrift,
     
    StarfallRain,
}

 
 
 
fn mode_value(mode: SkyMode, custom_loaded: bool) -> f32 {
    match mode {
        SkyMode::StarryNight => 0.0,
        SkyMode::BlackHole => 1.0,
        SkyMode::Custom if custom_loaded => 2.0,
        SkyMode::Custom => 0.0,
        SkyMode::IceNebula => 3.0,
        SkyMode::CrystalLattice => 4.0,
        SkyMode::FrostVeins => 5.0,
        SkyMode::AuroraVeil => 6.0,
        SkyMode::EmberDrift => 7.0,
        SkyMode::StarfallRain => 8.0,
    }
}

impl SkyMode {
     
     
    pub fn from_choice(index: usize) -> SkyMode {
        match index {
            1 => SkyMode::BlackHole,
            2 => SkyMode::Custom,
            3 => SkyMode::IceNebula,
            4 => SkyMode::CrystalLattice,
            5 => SkyMode::FrostVeins,
            6 => SkyMode::AuroraVeil,
            7 => SkyMode::EmberDrift,
            8 => SkyMode::StarfallRain,
            _ => SkyMode::StarryNight,
        }
    }
}

 
 
 
 
pub(crate) const DEFAULT_NOISE_COLOR: [f32; 3] = [0.855, 0.188, 0.212];

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyStyle {
    pub mode: SkyMode,
     
    pub density: f32,
     
    pub star_size: f32,
     
    pub twinkle: f32,
     
    pub glow: bool,
     
    pub hole: f32,
     
    pub spin: f32,
     
    pub noise_scale: f32,
     
    pub flow: f32,
     
    pub nebula_color: [f32; 3],
     
    pub lattice_color: [f32; 3],
     
    pub frost_color: [f32; 3],
     
    pub aurora_color: [f32; 3],
     
    pub ember_color: [f32; 3],
     
    pub rain_color: [f32; 3],
     
    pub brightness: f32,
}

impl Default for SkyStyle {
    fn default() -> Self {
        Self {
            mode: SkyMode::StarryNight,
            density: 1.0,
            star_size: 1.0,
            twinkle: 1.0,
            glow: true,
            hole: 0.24,
            spin: 0.35,
            noise_scale: 1.0,
            flow: 0.6,
            nebula_color: DEFAULT_NOISE_COLOR,
            lattice_color: DEFAULT_NOISE_COLOR,
            frost_color: DEFAULT_NOISE_COLOR,
            aurora_color: DEFAULT_NOISE_COLOR,
            ember_color: DEFAULT_NOISE_COLOR,
            rain_color: DEFAULT_NOISE_COLOR,
            brightness: 1.0,
        }
    }
}

 
 
static STYLE: Mutex<Option<SkyStyle>> = Mutex::new(None);
 
static GPU: Mutex<Option<Gpu>> = Mutex::new(None);
 
static CLOCK: OnceLock<Instant> = OnceLock::new();
 
 
 
 
static HOLE_ANCHOR: Mutex<Option<(SkyMode, [f32; 3])>> = Mutex::new(None);
 
 
static CUSTOM_IMAGE: Mutex<Option<PathBuf>> = Mutex::new(None);

 
struct CustomTexture {
    path: PathBuf,
    texture: u32,
}
static CUSTOM_TEXTURE: Mutex<Option<CustomTexture>> = Mutex::new(None);

struct LyricsTexture {
    texture: u32,
    revision: u64,
    heading: f32,
}
static LYRICS_TEXTURE: Mutex<Option<LyricsTexture>> = Mutex::new(None);
 
 
 
static BROKEN: AtomicBool = AtomicBool::new(false);
 
 
static PROGRAM_LOGGED: AtomicBool = AtomicBool::new(false);
 
static CUSTOM_FAILED: Mutex<Option<PathBuf>> = Mutex::new(None);

 
 
 
const SKY_PARALLAX: f32 = 0.0002;

 
 
pub(crate) const SHADER_NAME: &str = "atmosphere";

 
 
static TAIL_ACTIVE: AtomicBool = AtomicBool::new(false);
 
 
static TAIL_WARNED: AtomicBool = AtomicBool::new(false);

 
 
 
 
fn request_tail(active: bool) {
    if TAIL_ACTIVE.swap(active, Ordering::SeqCst) == active {
        return;
    }
    if let Err(error) = crate::camera_bridge::set_atmosphere_active(active) {
        if !TAIL_WARNED.swap(true, Ordering::Relaxed) {
            log::warn!(
                "Atmosphere: the world-render tail is unavailable, so the sky cannot draw: {error}"
            );
        }
         
        TAIL_ACTIVE.store(!active, Ordering::SeqCst);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
pub(crate) fn set_style(style: SkyStyle) {
    *lock(&STYLE) = Some(style);
    request_tail(true);
}

 
 
 
pub(crate) fn set_custom_image(path: Option<PathBuf>) {
    let mut current = lock(&CUSTOM_IMAGE);
    if *current == path {
        return;
    }
    *current = path;
     
    *lock(&CUSTOM_FAILED) = None;
}

 
 
 
pub(crate) fn clear_style() {
    *lock(&STYLE) = None;
    *lock(&HOLE_ANCHOR) = None;
    request_tail(false);
}

 
 
pub(crate) fn is_active() -> bool {
    lock(&STYLE).is_some()
}

pub(crate) fn sky_available() -> bool {
    is_active()
        && !is_broken()
        && TAIL_ACTIVE.load(Ordering::SeqCst)
        && (crate::jvmti::sky_drawn_in_sky_pass()
            || crate::jvmti::transform_installed(crate::jvmti::SMOKE_RENDER_TRANSFORM_PARAMS.log_label))
}

 
 
 
 
 
 
pub(crate) fn retry() {
    BROKEN.store(false, Ordering::SeqCst);
}

 
 
pub(crate) fn is_broken() -> bool {
    BROKEN.load(Ordering::SeqCst)
}

 
 
fn elapsed_seconds() -> f32 {
    CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32()
}

 
#[derive(Clone, Copy, Debug, PartialEq)]
struct SkyCamera {
    forward: [f32; 3],
     
    right: [f32; 3],
    up: [f32; 3],
     
     
    fov: f32,
     
    origin: [f32; 3],
}

impl Default for SkyCamera {
    fn default() -> Self {
         
        Self {
            forward: [0.0, 0.0, 1.0],
            right: [-1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 70.0,
            origin: [0.0; 3],
        }
    }
}

impl SkyCamera {
     
     
     
     
     
     
     
     
     
    fn is_sane(&self) -> bool {
        let finite = |v: [f32; 3]| v.iter().all(|c| c.is_finite());
        if !(finite(self.forward) && finite(self.right) && finite(self.up) && finite(self.origin)) {
            return false;
        }
        let length = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let forward = length(self.forward);
        let up = length(self.up);
        (0.5..=1.5).contains(&forward)
            && (0.5..=1.5).contains(&up)
            && self.fov.is_finite()
            && (1.0..=180.0).contains(&self.fov)
    }
}

 
 
 
 
 
 
 
 
 
 
 
fn read_camera() -> Option<SkyCamera> {
    let renderer = minecraft().game_renderer().ok()?;
    let camera = renderer.get_main_camera().ok()?;
    let (look, up, left) = camera.basis().ok()?;
    let origin = camera.position().ok()?;
     
     
     
    let partial_tick = camera.partial_tick_time().unwrap_or(1.0);
    let fov = renderer
        .get_fov(&camera, partial_tick, true)
        .unwrap_or(70.0);
    let fov = if fov.is_finite() && fov > 1.0 {
        fov
    } else {
        70.0
    };
    let camera = SkyCamera {
        forward: [look.x() as f32, look.y() as f32, look.z() as f32],
        right: [-left.x() as f32, -left.y() as f32, -left.z() as f32],
        up: [up.x() as f32, up.y() as f32, up.z() as f32],
        fov,
        origin: [origin.x() as f32, origin.y() as f32, origin.z() as f32],
    };
     
     
    camera.is_sane().then_some(camera)
}

 
 
fn current_camera() -> SkyCamera {
    static LAST: Mutex<Option<SkyCamera>> = Mutex::new(None);
    let mut last = lock(&LAST);
    if let Some(camera) = read_camera() {
        *last = Some(camera);
    }
    last.filter(|camera| camera.is_sane()).unwrap_or_default()
}

 
 
fn chart(direction: [f32; 3]) -> [f32; 2] {
    let scale = 2.0 / (1.0 + direction[1].max(-0.9));
    [direction[0] * scale, direction[2] * scale]
}

 
 
 
 
fn hole_centre(camera: &SkyCamera, mode: SkyMode) -> [f32; 2] {
    let mut anchor = lock(&HOLE_ANCHOR);
    let direction = match *anchor {
        Some((hung, direction)) if hung == mode => direction,
        _ => {
            *anchor = Some((mode, camera.forward));
            camera.forward
        }
    };
    let centre = chart(direction);
    [
        centre[0] + camera.origin[0] * SKY_PARALLAX,
        centre[1] + camera.origin[2] * SKY_PARALLAX,
    ]
}

struct Gpu {
    program: u32,
    vao: u32,
    vbo: u32,
     
     
     
     
     
    uniforms: [i32; 32],
}

impl Gpu {
     
     
     
     
     
     
     
     
     
     
    unsafe fn is_live(&self) -> bool {
        if gl::IsProgram(self.program) == 0 {
            return false;
        }
        let name = CString::new("u_hole_centre").expect("static uniform name");
        gl::GetUniformLocation(self.program, name.as_ptr()) == self.uniforms[18]
    }

     
     
    unsafe fn destroy(self) {
        gl::DeleteVertexArrays(1, &self.vao);
        gl::DeleteBuffers(1, &self.vbo);
    }

    unsafe fn create() -> anyhow::Result<Self> {
        let program = load_program()?;
        let mut uniforms = [0_i32; 32];
        const NAMES: [&str; 32] = [
            "u_time",
            "u_mode",
            "u_density",
            "u_star_size",
            "u_twinkle",
            "u_glow",
            "u_hole",
            "u_spin",
            "u_noise_scale",
            "u_flow",
            "u_brightness",
            "u_resolution",
            "u_forward",
            "u_right",
            "u_up",
            "u_fov",
            "u_origin",
            "u_parallax",
            "u_hole_centre",
            "u_custom",
            "u_nebula_color",
            "u_lattice_color",
            "u_frost_color",
            "u_aurora_color",
            "u_ember_color",
            "u_rain_color",
            "u_lyrics",
            "u_lyrics_alpha",
            "u_lyrics_progress",
            "u_lyrics_text",
            "u_lyrics_highlight",
            "u_lyrics_heading",
        ];
        for (slot, name) in uniforms.iter_mut().zip(NAMES) {
            let c_name = CString::new(name)?;
            *slot = gl::GetUniformLocation(program, c_name.as_ptr());
            ensure!(*slot >= 0, "atmosphere shader has no {name} uniform");
        }
        let position_name = CString::new("Position")?;
        let position = gl::GetAttribLocation(program, position_name.as_ptr());
        ensure!(position >= 0, "atmosphere shader has no Position attribute");

        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        let mut vbo = 0;
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (QUAD.len() * std::mem::size_of::<f32>()) as isize,
            QUAD.as_ptr().cast(),
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(
            position as u32,
            3,
            gl::FLOAT,
            gl::FALSE,
            0,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(position as u32);
        gl::BindVertexArray(0);

        Ok(Self {
            program,
            vao,
            vbo,
            uniforms,
        })
    }

     
    unsafe fn draw(
        &self,
        style: &SkyStyle,
        time: f32,
        width: f32,
        height: f32,
        camera: &SkyCamera,
        custom: Option<u32>,
        lyrics: Option<(&crate::graphic::lyrics_world::SkyFrame, u32, f32)>,
    ) {
        gl::UseProgram(self.program);
        let u = self.uniforms;
        gl::Uniform1f(u[0], time);
        gl::Uniform1f(u[1], mode_value(style.mode, custom.is_some()));
        gl::Uniform1f(u[2], style.density);
        gl::Uniform1f(u[3], style.star_size);
        gl::Uniform1f(u[4], style.twinkle);
        gl::Uniform1f(u[5], if style.glow { 1.0 } else { 0.0 });
        gl::Uniform1f(u[6], style.hole);
        gl::Uniform1f(u[7], style.spin);
        gl::Uniform1f(u[8], style.noise_scale);
        gl::Uniform1f(u[9], style.flow);
        gl::Uniform1f(u[10], style.brightness);
        gl::Uniform2f(u[11], width, height);
        let [fx, fy, fz] = camera.forward;
        gl::Uniform3f(u[12], fx, fy, fz);
        let [rx, ry, rz] = camera.right;
        gl::Uniform3f(u[13], rx, ry, rz);
        let [ux, uy, uz] = camera.up;
        gl::Uniform3f(u[14], ux, uy, uz);
        gl::Uniform1f(u[15], camera.fov);
        let [ox, oy, oz] = camera.origin;
        gl::Uniform3f(u[16], ox, oy, oz);
        gl::Uniform1f(u[17], SKY_PARALLAX);
        let [hx, hy] = hole_centre(camera, style.mode);
        gl::Uniform2f(u[18], hx, hy);
         
         
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, custom.unwrap_or(0));
        gl::Uniform1i(u[19], 0);
        let [nx, ny, nz] = style.nebula_color;
        gl::Uniform3f(u[20], nx, ny, nz);
        let [lx, ly, lz] = style.lattice_color;
        gl::Uniform3f(u[21], lx, ly, lz);
        let [fr, fg, fb] = style.frost_color;
        gl::Uniform3f(u[22], fr, fg, fb);
        let [ar, ag, ab] = style.aurora_color;
        gl::Uniform3f(u[23], ar, ag, ab);
        let [er, eg, eb] = style.ember_color;
        gl::Uniform3f(u[24], er, eg, eb);
        let [rr, rg, rb] = style.rain_color;
        gl::Uniform3f(u[25], rr, rg, rb);
        gl::Uniform1i(u[26], 1);
        if let Some((frame, texture, heading)) = lyrics {
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::BindSampler(1, 0);
            gl::Uniform1f(u[27], frame.alpha);
            gl::Uniform1f(u[28], if frame.word_highlight { 0.05 + 0.9 * frame.progress } else { 0.0 });
            gl::Uniform3f(u[29], frame.text[0], frame.text[1], frame.text[2]);
            gl::Uniform3f(u[30], frame.highlight[0], frame.highlight[1], frame.highlight[2]);
            gl::Uniform1f(u[31], heading);
        } else {
            gl::Uniform1f(u[27], 0.0);
            gl::Uniform1f(u[28], 0.0);
            gl::Uniform3f(u[29], 1.0, 1.0, 1.0);
            gl::Uniform3f(u[30], 1.0, 1.0, 1.0);
            gl::Uniform1f(u[31], 0.0);
        }
        gl::ActiveTexture(gl::TEXTURE0);

         
         
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::STENCIL_TEST);

        gl::BindVertexArray(self.vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
        gl::BindVertexArray(0);
    }
}

 
 
 
 
 
 
unsafe fn load_program() -> anyhow::Result<u32> {
    let bridge = crate::camera_bridge::smoke_bridge().context("Smoke bridge unavailable")?;
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let name = env.new_string(SHADER_NAME)?;
    let vertex = env.new_string(VERTEX)?;
    let fragment = env.new_string(FRAGMENT)?;
    let result = env.call_static_method(
        class,
        "reloadShader",
        "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            JValue::Object(minecraft().jni_ref().as_obj()),
            JValue::Object(&name),
            JValue::Object(&vertex),
            JValue::Object(&fragment),
        ],
    );
    let program = match result {
        Ok(value) => value.i()? as u32,
        Err(error) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            anyhow::bail!("ShaderLoader.loadShader failed: {error}");
        }
    };
    ensure!(
        program != 0 && gl::IsProgram(program) != 0,
        "ShaderLoader returned an invalid atmosphere program"
    );
     
     
     
    if !PROGRAM_LOGGED.swap(true, Ordering::Relaxed) {
        log::info!("Atmosphere: ShaderLoader ready: {} sky", SHADER_NAME);
    }
    Ok(program)
}

 
 
 
unsafe fn custom_texture() -> Option<u32> {
    let wanted = lock(&CUSTOM_IMAGE).clone()?;
    {
        let current = lock(&CUSTOM_TEXTURE);
        if let Some(current) = current.as_ref() {
            if current.path == wanted {
                return Some(current.texture);
            }
        }
    }
    if lock(&CUSTOM_FAILED).as_deref() == Some(wanted.as_path()) {
        return None;
    }
    match upload_picture(&wanted) {
        Ok(texture) => {
            let mut current = lock(&CUSTOM_TEXTURE);
            if let Some(previous) = current.replace(CustomTexture {
                path: wanted,
                texture,
            }) {
                gl::DeleteTextures(1, &previous.texture);
            }
            Some(texture)
        }
        Err(error) => {
            log::warn!(
                "Atmosphere: could not load sky picture {}: {error:#}",
                wanted.display()
            );
            *lock(&CUSTOM_FAILED) = Some(wanted);
            None
        }
    }
}

 
 
unsafe fn upload_picture(path: &Path) -> anyhow::Result<u32> {
    let bytes = std::fs::read(path).context("reading the sky picture")?;
    let image = image::load_from_memory(&bytes)
        .context("decoding the sky picture")?
        .to_rgba8();
    let (width, height) = image.dimensions();
    ensure!(width > 0 && height > 0, "the sky picture is empty");

    let mut texture = 0;
    gl::GenTextures(1, &mut texture);
    gl::BindTexture(gl::TEXTURE_2D, texture);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
     
     
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA8 as i32,
        width as i32,
        height as i32,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        image.as_raw().as_ptr().cast(),
    );
    Ok(texture)
}

 
 
 
 
 
 
 
pub(crate) fn render_sky_pass() -> anyhow::Result<()> {
    render(false)
}

 
 
 
pub(crate) fn render_world() -> anyhow::Result<()> {
    render(true)
}

 
 
 
fn fail(error: &anyhow::Error) {
    BROKEN.store(true, Ordering::SeqCst);
    log::warn!(
        "Atmosphere: the sky shader could not be built ({error:#}); drawing the vanilla sky until the module restarts"
    );
    if let Err(error) =
        crate::camera_bridge::set_sky_hidden(crate::camera_bridge::SkyHider::Atmosphere, false)
    {
        log::debug!("Atmosphere: restoring the vanilla sky failed: {error}");
    }
}

unsafe fn lyrics_texture(
    frame: &crate::graphic::lyrics_world::SkyFrame,
    camera: &SkyCamera,
) -> Option<(u32, f32)> {
    let mut slot = lock(&LYRICS_TEXTURE);
    if slot.as_ref().is_none_or(|texture| gl::IsTexture(texture.texture) == 0) {
        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        if texture == 0 {
            return None;
        }
        *slot = Some(LyricsTexture { texture, revision: u64::MAX, heading: 0.0 });
    }
    let texture = slot.as_mut()?;
    if texture.revision != frame.revision {
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, texture.texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        let unpack_names = [gl::UNPACK_ALIGNMENT, gl::UNPACK_ROW_LENGTH,
            gl::UNPACK_SKIP_PIXELS, gl::UNPACK_SKIP_ROWS];
        let mut unpack = [0_i32; 4];
        for (name, value) in unpack_names.into_iter().zip(unpack.iter_mut()) {
            gl::GetIntegerv(name, value);
        }
        gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
        gl::PixelStorei(gl::UNPACK_SKIP_PIXELS, 0);
        gl::PixelStorei(gl::UNPACK_SKIP_ROWS, 0);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::R8 as i32,
            crate::graphic::lyrics_world::SKY_WIDTH as i32,
            crate::graphic::lyrics_world::SKY_HEIGHT as i32, 0,
            gl::RED, gl::UNSIGNED_BYTE, frame.pixels.as_ptr().cast());
        for (name, value) in unpack_names.into_iter().zip(unpack) {
            gl::PixelStorei(name, value);
        }
        texture.revision = frame.revision;
        let [x, _, z] = camera.forward;
        if x * x + z * z > 1e-6 {
            texture.heading = z.atan2(x);
        }
    }
    Some((texture.texture, texture.heading))
}

fn render(depth_test: bool) -> anyhow::Result<()> {
     
     
    let style = { *lock(&STYLE) };
    let Some(style) = style else {
        return Ok(());
    };
    if is_broken() {
         
         
        return Ok(());
    }

    let mut gpu = lock(&GPU);
    unsafe {
         
         
        if !gl::GetIntegerv::is_loaded() {
            gl::load_with(super::platform::gl_proc_address);
        }
        let _guard = GlGuard::capture();
        gl::ActiveTexture(gl::TEXTURE1);
        let mut previous_lyrics_texture = 0;
        let mut previous_lyrics_sampler = 0;
        gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut previous_lyrics_texture);
        gl::GetIntegerv(gl::SAMPLER_BINDING, &mut previous_lyrics_sampler);
        gl::ActiveTexture(gl::TEXTURE0);
         
         
         
         
         
        if !gpu.as_ref().is_some_and(|gpu| gpu.is_live()) {
            let stale = gpu.take();
            if let Some(stale) = stale {
                stale.destroy();
                log::info!("Atmosphere: the sky program was lost; rebuilding it");
            }
            match Gpu::create() {
                Ok(fresh) => *gpu = Some(fresh),
                Err(error) => {
                    fail(&error);
                    return Ok(());
                }
            }
        }
        let gpu = gpu.as_mut().context("atmosphere GPU state missing")?;
        let mut viewport = [0_i32; 4];
        gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        let width = viewport[2].max(1) as f32;
        let height = viewport[3].max(1) as f32;
         
         
        let camera = current_camera();
        let custom = if style.mode == SkyMode::Custom {
            custom_texture()
        } else {
            None
        };
         
         
         
         
        if depth_test {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
        } else {
            gl::Disable(gl::DEPTH_TEST);
        }
        let lyrics = crate::graphic::lyrics_world::sky_frame();
        let texture = lyrics.as_ref().and_then(|frame| lyrics_texture(frame, &camera));
        gpu.draw(&style, elapsed_seconds(), width, height, &camera, custom,
            lyrics.as_ref().zip(texture).map(|(frame, (texture, heading))| (frame, texture, heading)));
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, previous_lyrics_texture as u32);
        gl::BindSampler(1, previous_lyrics_sampler as u32);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as TestMutex, MutexGuard as TestGuard};

     
     
    fn globals() -> TestGuard<'static, ()> {
        static LOCK: TestMutex<()> = TestMutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

     
     
     
    #[test]
    fn the_sky_loads_its_own_shader_program() {
        assert_ne!(SHADER_NAME, crate::graphic::smoke::SHADER_NAME);
        assert_eq!(SHADER_NAME, "atmosphere");
    }

     
     
     
     
     
    #[test]
    fn the_bridge_releases_only_its_own_pass_and_can_reload() {
        let bridge = include_str!("../../java/InferedSmokeBridge.java");
        assert!(
            bridge.contains("loaders.remove(WORLD_PASS)"),
            "release must close only its own pass's loader"
        );
        assert!(
            !bridge.contains("loaders.clear()"),
            "release must not close every pass's loader"
        );
        assert!(
            bridge.contains("public static synchronized int reloadShader("),
            "the sky rebuilds through reloadShader"
        );
    }

     
     
     
    #[test]
    fn the_starry_night_is_the_default_and_the_fallback() {
        assert_eq!(SkyMode::from_choice(0), SkyMode::StarryNight);
        assert_eq!(SkyMode::from_choice(1), SkyMode::BlackHole);
        assert_eq!(SkyMode::from_choice(2), SkyMode::Custom);
        assert_eq!(SkyMode::from_choice(3), SkyMode::IceNebula);
        assert_eq!(SkyMode::from_choice(4), SkyMode::CrystalLattice);
        assert_eq!(SkyMode::from_choice(5), SkyMode::FrostVeins);
        assert_eq!(SkyMode::from_choice(6), SkyMode::AuroraVeil);
        assert_eq!(SkyMode::from_choice(7), SkyMode::EmberDrift);
        assert_eq!(SkyMode::from_choice(8), SkyMode::StarfallRain);
        assert_eq!(SkyMode::from_choice(8), SkyMode::StarryNight);
        assert_eq!(SkyStyle::default().mode, SkyMode::StarryNight);
    }

     
     
    #[test]
    fn every_scene_has_a_distinct_shader_mode() {
        let modes = [
            (SkyMode::StarryNight, 0.0),
            (SkyMode::BlackHole, 1.0),
            (SkyMode::Custom, 2.0),
            (SkyMode::IceNebula, 3.0),
            (SkyMode::CrystalLattice, 4.0),
            (SkyMode::FrostVeins, 5.0),
            (SkyMode::AuroraVeil, 6.0),
            (SkyMode::EmberDrift, 7.0),
            (SkyMode::StarfallRain, 8.0),
        ];
        for (mode, expected) in modes {
            assert_eq!(mode_value(mode, true), expected, "{mode:?}");
        }
        assert_eq!(mode_value(SkyMode::Custom, false), 0.0);
        for (_, expected) in modes {
            assert!(
                FRAGMENT.contains(&format!("mode == {}", expected as i32)),
                "shader branch for mode {expected} is missing"
            );
        }
    }

     
     
    #[test]
    fn sky_lyrics_are_composited_after_every_atmosphere_mode() {
        let composite = FRAGMENT.find("texture(u_lyrics, lyricUV)").unwrap();
        for mode in 0..=8 {
            assert!(FRAGMENT.find(&format!("mode == {mode}")).unwrap() < composite);
        }
        assert!(FRAGMENT.contains("atan(dir.z, dir.x)"));
        assert!(FRAGMENT.contains("0.5 + delta / 1.35"));
        assert!(FRAGMENT.contains("asin(clamp(dir.y"));
    }

    #[test]
    fn every_uniform_the_gpu_sets_is_declared_in_the_shader() {
        for name in [
            "u_time",
            "u_mode",
            "u_density",
            "u_star_size",
            "u_twinkle",
            "u_glow",
            "u_hole",
            "u_spin",
            "u_noise_scale",
            "u_flow",
            "u_brightness",
            "u_resolution",
            "u_forward",
            "u_right",
            "u_up",
            "u_fov",
            "u_origin",
            "u_parallax",
            "u_hole_centre",
            "u_custom",
            "u_nebula_color",
            "u_lattice_color",
            "u_frost_color",
            "u_aurora_color",
            "u_ember_color",
            "u_rain_color",
            "u_lyrics",
            "u_lyrics_alpha",
            "u_lyrics_progress",
            "u_lyrics_text",
            "u_lyrics_highlight",
            "u_lyrics_heading",
        ] {
            assert!(FRAGMENT.contains(name), "missing uniform {name}");
        }
        assert!(FRAGMENT.contains("#version 150"));
        assert!(VERTEX.contains("in vec3 Position"));
    }

     
     
    #[test]
    fn publishing_and_clearing_controls_the_pass() {
        let _globals = globals();
        set_style(SkyStyle::default());
        assert!(is_active());
        clear_style();
        assert!(!is_active());
        assert!(render_world().is_ok(), "no style means an inert pass");
    }

     
     
    #[test]
    fn the_sky_is_charted_from_world_directions() {
        assert_eq!(chart([0.0, 1.0, 0.0]), [0.0, 0.0]);
         
        assert!((chart([0.0, 0.0, 1.0])[1] - 2.0).abs() < 1e-6);
        assert!((chart([1.0, 0.0, 0.0])[0] - 2.0).abs() < 1e-6);
         
        let nadir = chart([0.0, -1.0, 0.0]);
        assert!(nadir[0].is_finite() && nadir[1].is_finite());
    }

     
     
    fn ray(camera: &SkyCamera, ndc: [f32; 2]) -> [f32; 3] {
        let tan_half = (camera.fov.clamp(10.0, 170.0) * 0.5).to_radians().tan();
        let aspect = 16.0 / 9.0;
        let mut dir = [0.0_f32; 3];
        for axis in 0..3 {
            dir[axis] = camera.forward[axis]
                + camera.right[axis] * (ndc[0] * tan_half * aspect)
                + camera.up[axis] * (ndc[1] * tan_half);
        }
        let length = dir.iter().map(|v| v * v).sum::<f32>().sqrt();
        dir.map(|v| v / length)
    }

     
    fn view_angle(camera: &SkyCamera, ndc: [f32; 2]) -> f32 {
        let dir = ray(camera, ndc);
        let dot = (0..3)
            .map(|axis| camera.forward[axis] * dir[axis])
            .sum::<f32>()
            .clamp(-1.0, 1.0);
        dot.acos().to_degrees()
    }

     
     
     
     
     
    #[test]
    fn the_ray_spread_follows_the_vertical_fov() {
        let camera = SkyCamera {
            forward: [0.0, 0.0, 1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 70.0,
            origin: [0.0; 3],
        };
        assert!((view_angle(&camera, [0.0, 1.0]) - 35.0).abs() < 0.01);
        assert!((view_angle(&camera, [0.0, -1.0]) - 35.0).abs() < 0.01);
        let wide = SkyCamera {
            fov: 110.0,
            ..camera
        };
        assert!((view_angle(&wide, [0.0, 1.0]) - 55.0).abs() < 0.01);
        let narrow = SkyCamera {
            fov: 30.0,
            ..camera
        };
        assert!((view_angle(&narrow, [0.0, 1.0]) - 15.0).abs() < 0.01);
         
         
        assert!(view_angle(&camera, [1.0, 0.0]) > 35.0);
        assert!(view_angle(&camera, [1.0, 1.0]) > view_angle(&camera, [0.0, 1.0]));
    }

     
     
    #[test]
    fn the_shader_rays_follow_the_camera_not_the_screen() {
        assert!(FRAGMENT.contains("normalize(u_forward"));
        assert!(FRAGMENT.contains("u_right * (ndc.x * tanHalf * aspect)"));
        assert!(FRAGMENT.contains("u_up * (ndc.y * tanHalf)"));
        assert!(FRAGMENT.contains("dir.xz / (1.0 + max(dir.y, -0.90))"));
         
        assert!(FRAGMENT.contains("clamp(dir.y, 0.0, 1.0)"));
        assert!(!FRAGMENT.contains("(uv - 0.5) * vec2("));
    }

     
     
    #[test]
    fn the_custom_picture_is_mapped_from_world_directions() {
        assert!(FRAGMENT.contains("texture(u_custom"));
        assert!(FRAGMENT.contains("atan(dir.z, dir.x)"));
        assert!(FRAGMENT.contains("asin(clamp(dir.y, -1.0, 1.0))"));
        assert!(FRAGMENT.contains("int mode = int(u_mode + 0.5)"));
        assert!(FRAGMENT.contains("if (mode == 2)"));
    }

     
     
    #[test]
    fn the_noise_scenes_visualise_perlin_worley_and_ridged_fields() {
        for needed in [
            "float perlin(",
            "float perlin_fbm(",
            "vec2 worley(",
            "float ridged(",
             
            "vec3 noiseBody(vec3 accent)",
            "vec3 noiseEmber(vec3 accent)",
            "vec3 nebula = u_nebula_color;",
            "vec3 lattice = u_lattice_color;",
            "vec3 frost = u_frost_color;",
        ] {
            assert!(FRAGMENT.contains(needed), "missing {needed}");
        }
         
        for needed in ["u_noise_scale", "u_flow", "u_time * u_flow"] {
            assert!(FRAGMENT.contains(needed), "missing {needed}");
        }
    }

     
     
     
     
    #[test]
    fn a_degenerate_camera_reading_is_rejected() {
        let good = SkyCamera::default();
        assert!(good.is_sane());

        let nan_forward = SkyCamera {
            forward: [f32::NAN, 0.0, 1.0],
            ..good
        };
        assert!(!nan_forward.is_sane(), "a NaN forward is not a heading");

        let zero_basis = SkyCamera {
            forward: [0.0; 3],
            ..good
        };
        assert!(!zero_basis.is_sane(), "normalize((0,0,0)) is NaN");

        let infinite_origin = SkyCamera {
            origin: [0.0, f32::INFINITY, 0.0],
            ..good
        };
        assert!(!infinite_origin.is_sane());

        let bad_fov = SkyCamera {
            fov: f32::NAN,
            ..good
        };
        assert!(!bad_fov.is_sane());

         
         
        let unnormalised = SkyCamera {
            up: [0.0, 7.0, 0.0],
            ..good
        };
        assert!(!unnormalised.is_sane());
    }

     
     
     
    #[test]
    fn a_start_retries_a_failed_sky() {
        let _globals = globals();
        clear_style();
        BROKEN.store(true, Ordering::SeqCst);
        assert!(is_broken());
        retry();
        assert!(!is_broken(), "a new start retries the program");
    }

     
     
    #[test]
    fn the_default_camera_faces_south_with_a_right_handed_basis() {
        let camera = SkyCamera::default();
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!((dot(camera.forward, camera.forward) - 1.0).abs() < 1e-6);
        assert!(dot(camera.forward, camera.right).abs() < 1e-6);
        assert!(dot(camera.forward, camera.up).abs() < 1e-6);
        assert!(dot(camera.right, camera.up).abs() < 1e-6);
        let cross = [
            camera.forward[1] * camera.up[2] - camera.forward[2] * camera.up[1],
            camera.forward[2] * camera.up[0] - camera.forward[0] * camera.up[2],
            camera.forward[0] * camera.up[1] - camera.forward[1] * camera.up[0],
        ];
        for axis in 0..3 {
            assert!(
                (cross[axis] - camera.right[axis]).abs() < 1e-6,
                "screen-right must be forward × up"
            );
        }
    }

     
     
     
    #[test]
    fn the_black_hole_is_hung_once_and_released_with_the_style() {
        let _globals = globals();
        clear_style();
        let mut camera = SkyCamera::default();
        let hung = hole_centre(&camera, SkyMode::BlackHole);
        assert_eq!(hung, chart(camera.forward));

        camera.forward = [1.0, 0.0, 0.0];
        assert_eq!(
            hole_centre(&camera, SkyMode::BlackHole),
            hung,
            "looking away moves nothing"
        );

         
         
        assert_eq!(
            hole_centre(&camera, SkyMode::StarryNight),
            chart(camera.forward)
        );

        clear_style();
        let north = SkyCamera {
            forward: [0.0, 0.0, -1.0],
            ..camera
        };
        assert_ne!(
            hole_centre(&north, SkyMode::BlackHole),
            hung,
            "re-enabling re-hangs it where the player looks"
        );
    }
}
