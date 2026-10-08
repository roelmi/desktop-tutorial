// NEON enteros: operaciones aritméticas, lógicas, desplazamientos, comparaciones y selección en todos los tamaños de elemento.
#include "rt.h"
typedef union { uint64_t q[2]; uint8_t b[16]; } Raw;
static uint64_t fold(uint64_t h, const void *p) { const Raw *r = (const Raw *)p; h = mix(h, r->q[0]); return mix(h, r->q[1]); }

#define DEFINE(NAME, ST, UT, E, SH)                                                              \
  typedef ST NAME##_s __attribute__((vector_size(16)));                                           \
  typedef UT NAME##_u __attribute__((vector_size(16)));                                           \
  static uint64_t test_##NAME(uint64_t h, uint64_t *s) {                                          \
    for (int it = 0; it < 40; it++) {                                                             \
      union { uint64_t q[2]; NAME##_s s; NAME##_u u; } A, B, M;                                   \
      A.q[0] = rng(s); A.q[1] = rng(s); B.q[0] = rng(s); B.q[1] = rng(s);                         \
      NAME##_s a = A.s, b = B.s; NAME##_u ua = A.u, ub = B.u;                                     \
      NAME##_s r;                                                                                 \
      r = a + b; h = fold(h, &r); r = a - b; h = fold(h, &r); r = -a; h = fold(h, &r);            \
      r = a & b; h = fold(h, &r); r = a | b; h = fold(h, &r); r = a ^ b; h = fold(h, &r);         \
      r = ~a; h = fold(h, &r); r = a & ~b; h = fold(h, &r); r = a | ~b; h = fold(h, &r);          \
      r = a << 3; h = fold(h, &r); r = a >> 2; h = fold(h, &r);                                   \
      NAME##_u ur = ua >> (SH - 1); h = fold(h, &ur); ur = ua << 1; h = fold(h, &ur);             \
      NAME##_s cnt = (NAME##_s)(ub & ((NAME##_u){} + (UT)(SH - 1)));                                         \
      r = a << cnt; h = fold(h, &r); ur = ua << (NAME##_u)cnt; h = fold(h, &ur);                  \
      ur = ua >> (NAME##_u)cnt; h = fold(h, &ur); r = a >> cnt; h = fold(h, &r);                  \
      M.s = (a < b); h = fold(h, &M); M.s = (a == b); h = fold(h, &M); M.s = (a > b); h = fold(h, &M);     \
      M.s = (a <= b); h = fold(h, &M); M.s = (a >= b); h = fold(h, &M); M.s = (a != b); h = fold(h, &M);   \
      NAME##_u uc = (ua < ub); h = fold(h, &uc); uc = (ua > ub); h = fold(h, &uc);                \
      NAME##_s m = a < b; r = (m & a) | (~m & b); h = fold(h, &r);                                \
      m = a > b; r = (m & a) | (~m & b); h = fold(h, &r);                                         \
      NAME##_s neg = (a < (NAME##_s){0}); r = (neg & -a) | (~neg & a); h = fold(h, &r);           \
      if (sizeof(E) != 8) { r = a * b; h = fold(h, &r); ur = ua * ub; h = fold(h, &ur); }         \
      r = a + 1; h = fold(h, &r); r = a - 1; h = fold(h, &r); r = a + (NAME##_s){}; h = fold(h, &r);   \
    }                                                                                             \
    return h;                                                                                     \
  }
DEFINE(v16, int8_t, uint8_t, int8_t, 8)
DEFINE(v8, int16_t, uint16_t, int16_t, 16)
DEFINE(v4, int32_t, uint32_t, int32_t, 32)
DEFINE(v2, int64_t, uint64_t, int64_t, 64)

uint64_t run(void) {
  uint64_t h = 23, s = 31337;
  h = test_v16(h, &s); h = test_v8(h, &s); h = test_v4(h, &s); h = test_v2(h, &s);
  return h;
}
