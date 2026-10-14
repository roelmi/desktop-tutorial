#include "rt.h"
typedef union { double d; uint64_t u; } D;
typedef union { float f; uint32_t u; } F;
static uint64_t bd(double d) { D x; x.d = d; return x.u; }
static uint64_t bf(float f) { F x; x.f = f; return x.u; }
uint64_t run(void) {
  uint64_t h = 19, s = 2024;
  for (int i = 0; i < 1200; i++) {
    double a = (double)(int64_t)(rng(&s) >> 11) / 1048576.0;
    double b = (double)(int32_t)rng(&s) / 4096.0 + 0.5;
    if (b == 0.0) b = 1.0;
    float fa = (float)(a / 1024.0), fb = (float)b;
    if (fb == 0.0f) fb = 1.0f;
    h = mix(h, bd(a + b)); h = mix(h, bd(a - b)); h = mix(h, bd(a * b)); h = mix(h, bd(a / b));
    h = mix(h, bd(__builtin_sqrt(__builtin_fabs(a)))); h = mix(h, bd(__builtin_fma(a, b, a))); h = mix(h, bd(-a));
    h = mix(h, a < b); h = mix(h, a == b); h = mix(h, a >= b); h = mix(h, bd(a < b ? a : b)); h = mix(h, bd(a > b ? a : b));
    h = mix(h, bf(fa + fb)); h = mix(h, bf(fa - fb)); h = mix(h, bf(fa * fb)); h = mix(h, bf(fa / fb));
    h = mix(h, bf(__builtin_sqrtf(__builtin_fabsf(fa)))); h = mix(h, bf(__builtin_fmaf(fa, fb, fa))); h = mix(h, bf(-fb));
    h = mix(h, fa < fb); h = mix(h, fa == fb); h = mix(h, fa <= fb); h = mix(h, bf(fa < fb ? fa : fb));
    h = mix(h, bd(__builtin_floor(b))); h = mix(h, bd(__builtin_ceil(b))); h = mix(h, bd(__builtin_trunc(b)));
    h = mix(h, bd(__builtin_round(b))); h = mix(h, bd(__builtin_rint(b)));
    h = mix(h, bf(__builtin_floorf(fb))); h = mix(h, bf(__builtin_ceilf(fb))); h = mix(h, bf(__builtin_truncf(fb))); h = mix(h, bf(__builtin_roundf(fb)));
    h = mix(h, (uint64_t)(int64_t)a); h = mix(h, (uint64_t)(int32_t)b); h = mix(h, (uint32_t)(b < 0 ? -b : b)); h = mix(h, (uint64_t)(a < 0 ? -a : a));
    h = mix(h, (uint64_t)(int32_t)fb); h = mix(h, (uint64_t)(int64_t)fb);
    int64_t si = (int64_t)rng(&s); uint64_t ui = rng(&s); int32_t wi = (int32_t)rng(&s); uint32_t wu = (uint32_t)rng(&s);
    h = mix(h, bd((double)si)); h = mix(h, bd((double)ui)); h = mix(h, bd((double)wi)); h = mix(h, bd((double)wu));
    h = mix(h, bf((float)si)); h = mix(h, bf((float)ui)); h = mix(h, bf((float)wi)); h = mix(h, bf((float)wu));
    h = mix(h, bd((double)fb)); h = mix(h, bf((float)a));
  }
  return h;
}
