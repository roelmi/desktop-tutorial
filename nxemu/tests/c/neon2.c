// NEON de coma flotante: float32x4 y float64x2.
#include "rt.h"
typedef float f4 __attribute__((vector_size(16)));
typedef double d2 __attribute__((vector_size(16)));
typedef int32_t i4 __attribute__((vector_size(16)));
typedef uint32_t u4 __attribute__((vector_size(16)));
typedef int64_t l2 __attribute__((vector_size(16)));
typedef union { f4 f; uint32_t u[4]; uint64_t q[2]; } F4;
typedef union { d2 d; uint64_t q[2]; } D2;
static uint64_t foldf(uint64_t h, f4 v) { F4 x; x.f = v; h = mix(h, x.q[0]); return mix(h, x.q[1]); }
static uint64_t foldd(uint64_t h, d2 v) { D2 x; x.d = v; h = mix(h, x.q[0]); return mix(h, x.q[1]); }
static f4 rf(uint64_t *s) { F4 x; for (int k = 0; k < 4; k++) x.f[k] = (float)(int32_t)(rng(s) >> 40) / 1024.0f + 0.25f; return x.f; }
static d2 rd(uint64_t *s) { D2 x; for (int k = 0; k < 2; k++) x.d[k] = (double)(int64_t)(rng(s) >> 30) / 65536.0 + 0.125; return x.d; }
uint64_t run(void) {
  uint64_t h = 29, s = 4242;
  for (int it = 0; it < 120; it++) {
    f4 a = rf(&s), b = rf(&s), c = rf(&s);
    h = foldf(h, a + b); h = foldf(h, a - b); h = foldf(h, a * b); h = foldf(h, a / b); h = foldf(h, -a);
    h = foldf(h, a * 2.5f); h = foldf(h, a + 1.0f); h = foldf(h, __builtin_elementwise_sqrt(__builtin_elementwise_abs(a)));
    h = foldf(h, __builtin_elementwise_fma(a, b, c)); h = foldf(h, __builtin_elementwise_abs(a));
    h = foldf(h, __builtin_elementwise_min(a, b)); h = foldf(h, __builtin_elementwise_max(a, b));
    i4 m1 = a < b, m2 = a > b, m3 = a == b, m4 = a >= b; h = mix(h, (uint64_t)m1[0] ^ ((uint64_t)m2[1] << 7) ^ ((uint64_t)m3[2] << 9) ^ ((uint64_t)m4[3] << 11));
    F4 sel; sel.f = (f4)((m1 & (i4)a) | (~m1 & (i4)b)); h = mix(h, sel.q[0]); h = mix(h, sel.q[1]);
    h = foldf(h, __builtin_elementwise_floor(a)); h = foldf(h, __builtin_elementwise_ceil(a)); h = foldf(h, __builtin_elementwise_trunc(a));
    i4 ci = __builtin_convertvector(a, i4); h = mix(h, (uint32_t)ci[0] ^ ((uint64_t)(uint32_t)ci[1] << 32)); h = mix(h, (uint32_t)ci[2] ^ ((uint64_t)(uint32_t)ci[3] << 32));
    f4 cf = __builtin_convertvector(ci, f4); h = foldf(h, cf);
    u4 cu = __builtin_convertvector(__builtin_elementwise_abs(a), u4); f4 cf2 = __builtin_convertvector(cu, f4); h = foldf(h, cf2);
    f4 sw = __builtin_shufflevector(a, b, 0, 4, 1, 5); h = foldf(h, sw); sw = __builtin_shufflevector(a, a, 3, 2, 1, 0); h = foldf(h, sw);
    h = foldf(h, (f4){a[0], a[0], a[0], a[0]});
    float sum = a[0] + a[1] + a[2] + a[3]; F4 t; t.f = (f4){sum, 0, 0, 0}; h = mix(h, t.u[0]);
    d2 x = rd(&s), y = rd(&s), z = rd(&s);
    h = foldd(h, x + y); h = foldd(h, x - y); h = foldd(h, x * y); h = foldd(h, x / y); h = foldd(h, -x);
    h = foldd(h, __builtin_elementwise_sqrt(__builtin_elementwise_abs(x))); h = foldd(h, __builtin_elementwise_fma(x, y, z)); h = foldd(h, __builtin_elementwise_abs(x));
    h = foldd(h, __builtin_elementwise_min(x, y)); h = foldd(h, __builtin_elementwise_max(x, y));
    l2 mm = x < y; h = mix(h, (uint64_t)mm[0] ^ ((uint64_t)mm[1] << 5));
    l2 cl = __builtin_convertvector(x, l2); h = mix(h, (uint64_t)cl[0]); h = mix(h, (uint64_t)cl[1]);
    d2 cd = __builtin_convertvector(cl, d2); h = foldd(h, cd);
    d2 swd = __builtin_shufflevector(x, y, 1, 2); h = foldd(h, swd);
    f4 narrowed = __builtin_shufflevector(__builtin_convertvector(x, __attribute__((vector_size(8))) float), __builtin_convertvector(y, __attribute__((vector_size(8))) float), 0, 1, 2, 3);
    h = foldf(h, narrowed);
  }
  return h;
}
