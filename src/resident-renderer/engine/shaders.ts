/**
 * Story 7.5. 상주 WebGL2 엔진의 GLSL 원문.
 *
 * **이 shader들은 darktable의 근사다. 동등하다고 주장하지 않는다.**
 * 동등성은 여기서 선언되는 것이 아니라 T6의 지표와 사람 눈 검토로 **측정**된다.
 * 지표가 통과하지 못하면 후보는 기본 비활성 상태를 유지한다.
 *
 * 프로그램 집합이 preset과 무관하게 **고정**인 것이 중요하다. preset마다 shader를
 * 만들면 새 preset이 hot path에서 컴파일을 유발하고, 그 순간 "상주"라는 말이 거짓이 된다.
 * 그래서 연산 on/off는 컴파일 분기가 아니라 uniform으로 처리한다.
 */

export const RESIDENT_VERTEX_SHADER = `#version 300 es
layout(location = 0) in vec2 aPosition;
out vec2 vUv;

void main() {
  vUv = aPosition * 0.5 + 0.5;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`

/**
 * 점 단위 연산 한 묶음: temperature → exposure → sigmoid → monochrome.
 *
 * darktable의 순서를 그대로 따른다. 순서를 바꾸면 같은 파라미터로 다른 색이 나온다.
 */
export const RESIDENT_POINTWISE_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uSource;

uniform bool uTemperatureEnabled;
uniform vec3 uTemperatureCoeffs;

uniform bool uExposureEnabled;
uniform float uExposureBlack;
uniform float uExposureEv;

uniform bool uSigmoidEnabled;
uniform float uSigmoidContrast;
uniform float uSigmoidSkew;
uniform float uSigmoidWhiteTarget;
uniform float uSigmoidBlackTarget;

uniform bool uMonochromeEnabled;
uniform float uMonochromeHighlights;

const float MIDDLE_GREY = 0.1845;

vec3 srgbToLinear(vec3 color) {
  bvec3 cutoff = lessThanEqual(color, vec3(0.04045));
  vec3 low = color / 12.92;
  vec3 high = pow((color + 0.055) / 1.055, vec3(2.4));
  return mix(high, low, vec3(cutoff));
}

vec3 linearToSrgb(vec3 color) {
  bvec3 cutoff = lessThanEqual(color, vec3(0.0031308));
  vec3 low = color * 12.92;
  vec3 high = 1.055 * pow(color, vec3(1.0 / 2.4)) - 0.055;
  return mix(high, low, vec3(cutoff));
}

/**
 * darktable sigmoid의 per-channel 곡선.
 *
 * 논문 그대로가 아니라 문서화된 형태(대비 기울기 + 왜도)를 따른 근사이며,
 * middle grey를 고정점으로 유지한다.
 */
float sigmoidCurve(float value) {
  float white = max(uSigmoidWhiteTarget * 0.01, 1e-4);
  float black = max(uSigmoidBlackTarget * 0.01, 0.0);
  float x = max(value, 1e-6);

  // middle grey 기준 log 공간으로 옮긴다.
  float logValue = log2(x / MIDDLE_GREY);
  // 왜도는 어두운 쪽과 밝은 쪽의 기울기를 비대칭으로 만든다.
  float slope = uSigmoidContrast * (logValue < 0.0 ? 1.0 + uSigmoidSkew : 1.0 - uSigmoidSkew);
  float shaped = logValue * slope;
  float normalized = shaped / (1.0 + abs(shaped));

  return clamp(black + (white - black) * (normalized * 0.5 + 0.5), 0.0, 1.0);
}

void main() {
  vec3 color = srgbToLinear(texture(uSource, vUv).rgb);

  if (uTemperatureEnabled) {
    color *= uTemperatureCoeffs;
  }

  if (uExposureEnabled) {
    color = (color - vec3(uExposureBlack)) * exp2(uExposureEv);
  }

  color = max(color, vec3(0.0));

  if (uSigmoidEnabled) {
    color = vec3(sigmoidCurve(color.r), sigmoidCurve(color.g), sigmoidCurve(color.b));
  }

  if (uMonochromeEnabled) {
    // darktable monochrome은 Lab a/b 평면의 가우시안 가중이다.
    // 여기서는 밝기 보존 그레이스케일 + highlights 보정으로 근사한다.
    float luma = dot(color, vec3(0.2126, 0.7152, 0.0722));
    float lifted = mix(luma, 1.0 - (1.0 - luma) * (1.0 - luma), clamp(uMonochromeHighlights, 0.0, 1.0));
    color = vec3(lifted);
  }

  fragColor = vec4(linearToSrgb(clamp(color, 0.0, 1.0)), 1.0);
}
`

/** 분리형 가우시안 blur 한 방향. bloom과 sharpen이 함께 쓴다. */
export const RESIDENT_BLUR_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uSource;
uniform vec2 uTexelStep;
uniform float uRadius;
uniform bool uThresholdEnabled;
uniform float uThreshold;

void main() {
  float sigma = max(uRadius, 0.5);
  float twoSigmaSquared = 2.0 * sigma * sigma;
  int taps = int(clamp(ceil(sigma * 2.0), 1.0, 24.0));

  vec3 accumulated = vec3(0.0);
  float weightSum = 0.0;

  for (int offset = -24; offset <= 24; offset++) {
    if (offset < -taps || offset > taps) {
      continue;
    }

    float distance = float(offset);
    float weight = exp(-(distance * distance) / twoSigmaSquared);
    vec3 sampled = texture(uSource, vUv + uTexelStep * distance).rgb;

    if (uThresholdEnabled) {
      float luma = dot(sampled, vec3(0.2126, 0.7152, 0.0722));
      sampled = luma >= uThreshold ? sampled : vec3(0.0);
    }

    accumulated += sampled * weight;
    weightSum += weight;
  }

  fragColor = vec4(accumulated / max(weightSum, 1e-6), 1.0);
}
`

/** bloom 합성. 임계값을 넘은 밝은 부분을 screen 블렌드로 얹는다. */
export const RESIDENT_BLOOM_COMPOSITE_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uSource;
uniform sampler2D uBloom;
uniform float uStrength;

void main() {
  vec3 base = texture(uSource, vUv).rgb;
  vec3 glow = texture(uBloom, vUv).rgb * uStrength;
  vec3 screened = 1.0 - (1.0 - base) * (1.0 - clamp(glow, 0.0, 1.0));

  fragColor = vec4(clamp(screened, 0.0, 1.0), 1.0);
}
`

/** unsharp mask. threshold 아래의 차이는 노이즈로 보고 증폭하지 않는다. */
export const RESIDENT_SHARPEN_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uSource;
uniform sampler2D uBlurred;
uniform float uAmount;
uniform float uThreshold;

void main() {
  vec3 base = texture(uSource, vUv).rgb;
  vec3 blurred = texture(uBlurred, vUv).rgb;
  vec3 detail = base - blurred;
  vec3 gated = mix(vec3(0.0), detail, step(vec3(uThreshold), abs(detail)));

  fragColor = vec4(clamp(base + gated * uAmount, 0.0, 1.0), 1.0);
}
`

/** 최종 출력 복사. */
export const RESIDENT_BLIT_SHADER = `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uSource;

void main() {
  fragColor = vec4(texture(uSource, vUv).rgb, 1.0);
}
`

/**
 * 엔진이 prewarm에서 **전부** 컴파일하는 프로그램 목록.
 *
 * preset이 늘어나도 이 목록은 늘지 않는다. 그래야 새 preset이 hot path에
 * 컴파일을 끌고 들어오지 않는다.
 */
export const RESIDENT_PROGRAM_SOURCES = {
  pointwise: RESIDENT_POINTWISE_SHADER,
  blur: RESIDENT_BLUR_SHADER,
  bloomComposite: RESIDENT_BLOOM_COMPOSITE_SHADER,
  sharpen: RESIDENT_SHARPEN_SHADER,
  blit: RESIDENT_BLIT_SHADER,
} as const

export type ResidentProgramName = keyof typeof RESIDENT_PROGRAM_SOURCES
