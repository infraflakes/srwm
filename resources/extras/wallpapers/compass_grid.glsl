// Compass dot grid — white dots over a directional color gradient.
// Background hue shifts by cardinal direction from the canvas origin:
//   North → blue, South → red, East → yellow, West → green.
// Diagonals are natural blends (NE = teal-ish, SW = orange-ish, etc.).
// Helps you feel where you are on the infinite canvas.
// Zoom is handled externally (RescaleRenderElement) — shader works in canvas space.
precision highp float;

varying vec2 v_coords;
uniform vec2 size;
uniform float alpha;

uniform vec2 u_camera;

// --- Tweak these ---
const vec4 DOT_COLOR = vec4(1.0, 1.0, 1.0, 1.0);
const float DOT_SPACING = 80.0; // canvas pixels between dots
const float DOT_RADIUS = 1.0;   // dot radius in canvas pixels

// How far (canvas pixels) before directional tint approaches full strength.
const float GRADIENT_SCALE = 1000.0;

// How strong the directional tint is (0.0 = invisible, 1.0 = vivid).
const float TINT_STRENGTH = 0.45;

// Base dark color at the origin
const vec3 BASE_COLOR = vec3(0.06, 0.06, 0.08);

// Radial brightness waves — concentric ripples from the origin.
const float WAVE_PERIOD = 2000.0;   // pixels between wave peaks
const float WAVE_STRENGTH = 0.035;  // brightness swing (keep subtle)

// Cardinal direction colors (blend naturally at diagonals)
const vec3 NORTH_COLOR = vec3(0.2, 0.35, 0.9);  // blue
const vec3 SOUTH_COLOR = vec3(0.9, 0.2, 0.2);   // red
const vec3 EAST_COLOR  = vec3(0.9, 0.8, 0.15);  // yellow
const vec3 WEST_COLOR  = vec3(0.15, 0.8, 0.3);  // green
// -------------------

void main() {
    // Canvas position: size is the visible canvas area, u_camera is the offset
    vec2 canvas_pos = v_coords * size + u_camera;

    // --- Directional color gradient ---
    // Sigmoid: maps (-inf, inf) -> (-1, 1) smoothly
    vec2 norm = canvas_pos / (GRADIENT_SCALE + abs(canvas_pos));

    // Cardinal weights (each 0 to 1)
    float north = max(-norm.y, 0.0);
    float south = max( norm.y, 0.0);
    float east  = max( norm.x, 0.0);
    float west  = max(-norm.x, 0.0);

    vec3 tint = north * NORTH_COLOR
              + south * SOUTH_COLOR
              + east  * EAST_COLOR
              + west  * WEST_COLOR;

    // Radial brightness waves (concentric rings from origin)
    float dist = length(canvas_pos);
    float wave_phase = mod(dist, WAVE_PERIOD);
    float wave = sin(wave_phase / WAVE_PERIOD * 6.2832) * WAVE_STRENGTH;

    vec3 bg = BASE_COLOR + tint * TINT_STRENGTH + wave;

    // --- Dot grid ---
    vec2 canvas_mod = mod(canvas_pos, DOT_SPACING);
    vec2 dist_to_dot = min(canvas_mod, DOT_SPACING - canvas_mod);
    float d = length(dist_to_dot);
    float dot_alpha = 1.0 - smoothstep(DOT_RADIUS - 0.5, DOT_RADIUS + 0.5, d);

    gl_FragColor = vec4(mix(bg, DOT_COLOR.rgb, dot_alpha), 1.0) * alpha;
}
