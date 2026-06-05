#version 100
precision lowp float;

varying vec2 uv;
uniform sampler2D Texture;

uniform vec2 Resolution;
uniform float Intensity;
uniform float CrtIntensity;
uniform vec4 _Time;

void main() {
  // Пиксельные координаты
  vec2 U = uv * Resolution;

  float US1 = sin(U.y) / 2.0 + 0.7;

  vec3 US2;
  US2.x = sin(20.0 * uv.y + (-_Time.x * 2.0 - 0.4)) / 10.0 + 0.85; // R-волна
  US2.y = sin(20.0 * uv.y + (-_Time.x * 2.0)) / 10.0 + 0.85; //       G-волна
  US2.z = sin(20.0 * uv.y + (-_Time.x * 2.0 + 0.4)) / 10.0 + 0.85; // B-волна

  // Объединяем полосы и волны помех
  vec3 US = US1 * US2;

  vec2 UR = uv + vec2(0.01 * Intensity, 0.0);
  vec2 UG = uv + vec2(0.0, 0.0);
  vec2 UB = uv + vec2(-0.01 * Intensity, 0.0);

  float base_alpha = texture2D(Texture, uv).a;

  vec4 CR = texture2D(Texture, UR) * vec4(0.8, 0.1, 0.1, 1.0);
  vec4 CG = texture2D(Texture, UG) * vec4(0.1, 0.8, 0.1, 1.0);
  vec4 CB = texture2D(Texture, UB) * vec4(0.1, 0.1, 0.8, 1.0);

  // Падение контраста
  vec4 CL = vec4(1.2, 1.2, 1.2, 1.0);

  vec4 O = (CR + CB + CG) / CL;
  // Накладываем волны помех и делаем цвета чуть сочнее
  O.rgb *= mix(vec3(1.0), US * 1.1, CrtIntensity);
  O.a = base_alpha;

  gl_FragColor = O;
}
