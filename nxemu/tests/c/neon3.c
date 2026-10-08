// NEON: reordenaciones, ensanchado/estrechado, carriles, saturación, ordenaciones por byte.
#include "rt.h"
#ifndef ONLY
#define ONLY (-1)
#endif
#define G(n) if (ONLY & (1 << (n)))
typedef uint8_t u8x16 __attribute__((vector_size(16)));
typedef int8_t s8x16 __attribute__((vector_size(16)));
typedef uint16_t u16x8 __attribute__((vector_size(16)));
typedef int16_t s16x8 __attribute__((vector_size(16)));
typedef uint32_t u32x4 __attribute__((vector_size(16)));
typedef int32_t s32x4 __attribute__((vector_size(16)));
typedef uint64_t u64x2 __attribute__((vector_size(16)));
typedef uint8_t u8x8 __attribute__((vector_size(8)));
typedef int16_t s16x4 __attribute__((vector_size(8)));
typedef uint16_t u16x4 __attribute__((vector_size(8)));
typedef uint32_t u32x2 __attribute__((vector_size(8)));
typedef union { u8x16 b; u16x8 h; u32x4 w; u64x2 d; uint64_t q[2]; } V;
static uint64_t fv(uint64_t h, V v) { h = mix(h, v.q[0]); return mix(h, v.q[1]); }
static V rv(uint64_t *s) { V v; v.q[0] = rng(s); v.q[1] = rng(s); return v; }
uint64_t run(void) {
  uint64_t h = 37, s = 5150;
  for (int it = 0; it < 60; it++) {
    V a = rv(&s), b = rv(&s), r;
    G(0) { r.b = __builtin_shufflevector(a.b, b.b, 0, 16, 1, 17, 2, 18, 3, 19, 4, 20, 5, 21, 6, 22, 7, 23); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, b.b, 8, 24, 9, 25, 10, 26, 11, 27, 12, 28, 13, 29, 14, 30, 15, 31); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, b.b, 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, b.b, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, b.b, 0, 16, 2, 18, 4, 20, 6, 22, 8, 24, 10, 26, 12, 28, 14, 30); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, b.b, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18); h = fv(h, r);
    r.b = __builtin_shufflevector(a.b, a.b, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0); h = fv(h, r);
    } G(1) { r.w = __builtin_shufflevector(a.w, b.w, 0, 4, 1, 5); h = fv(h, r); r.w = __builtin_shufflevector(a.w, b.w, 2, 6, 3, 7); h = fv(h, r);
    r.w = __builtin_shufflevector(a.w, b.w, 0, 2, 4, 6); h = fv(h, r); r.w = __builtin_shufflevector(a.w, b.w, 1, 3, 5, 7); h = fv(h, r);
    r.w = __builtin_shufflevector(a.w, a.w, 2, 2, 2, 2); h = fv(h, r); r.w = __builtin_shufflevector(a.w, a.w, 1, 0, 3, 2); h = fv(h, r);
    } G(2) { r.d = __builtin_shufflevector(a.d, b.d, 0, 2); h = fv(h, r); r.d = __builtin_shufflevector(a.d, b.d, 1, 3); h = fv(h, r);
    r.h = __builtin_shufflevector(a.h, b.h, 0, 8, 1, 9, 2, 10, 3, 11); h = fv(h, r); r.h = __builtin_shufflevector(a.h, b.h, 1, 3, 5, 7, 9, 11, 13, 15); h = fv(h, r);
    } G(3) {
    u16x8 w16 = __builtin_convertvector(__builtin_shufflevector(a.b, a.b, 0, 1, 2, 3, 4, 5, 6, 7), u16x8); r.h = w16; h = fv(h, r);
    s16x8 s16 = __builtin_convertvector(__builtin_shufflevector((s8x16)a.b, (s8x16)a.b, 8, 9, 10, 11, 12, 13, 14, 15), s16x8); r.h = (u16x8)s16; h = fv(h, r);
    u32x4 w32 = __builtin_convertvector(__builtin_shufflevector(a.h, a.h, 0, 1, 2, 3), u32x4); r.w = w32; h = fv(h, r);
    s32x4 s32 = __builtin_convertvector(__builtin_shufflevector((s16x8)a.h, (s16x8)a.h, 4, 5, 6, 7), s32x4); r.w = (u32x4)s32; h = fv(h, r);
    } G(4) { u8x8 n8 = __builtin_convertvector(a.h, u8x8); r.q[0] = *(uint64_t *)&n8; r.q[1] = 0; h = fv(h, r);
    u16x4 n16 = __builtin_convertvector(a.w, u16x4); r.q[0] = *(uint64_t *)&n16; r.q[1] = 0; h = fv(h, r);
    u32x2 n32 = __builtin_convertvector(a.d, u32x2); r.q[0] = *(uint64_t *)&n32; r.q[1] = 0; h = fv(h, r);
    } G(5) {
    r.b = __builtin_elementwise_add_sat(a.b, b.b); h = fv(h, r); r.b = __builtin_elementwise_sub_sat(a.b, b.b); h = fv(h, r);
    r.h = (u16x8)__builtin_elementwise_add_sat((s16x8)a.h, (s16x8)b.h); h = fv(h, r); r.w = (u32x4)__builtin_elementwise_sub_sat((s32x4)a.w, (s32x4)b.w); h = fv(h, r);
    } G(6) { r.b = __builtin_elementwise_min(a.b, b.b); h = fv(h, r); r.b = __builtin_elementwise_max(a.b, b.b); h = fv(h, r);
    r.h = (u16x8)__builtin_elementwise_min((s16x8)a.h, (s16x8)b.h); h = fv(h, r); r.w = (u32x4)__builtin_elementwise_max((s32x4)a.w, (s32x4)b.w); h = fv(h, r);
    } G(7) { r.b = (u8x16)__builtin_elementwise_abs((s8x16)a.b); h = fv(h, r); r.w = (u32x4)__builtin_elementwise_abs((s32x4)a.w); h = fv(h, r);
    } G(8) {
    h = mix(h, a.b[3]); h = mix(h, (uint64_t)(int64_t)(int8_t)a.b[9]); h = mix(h, a.h[5]); h = mix(h, a.w[2]); h = mix(h, a.d[1]);
    r = a; r.b[5] = (uint8_t)b.q[1]; r.w[3] = (uint32_t)b.q[0]; r.h[2] = (uint16_t)b.q[1]; r.d[0] = b.d[1]; h = fv(h, r);
    } G(9) { r.b = (u8x16){b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0], b.b[0]}; h = fv(h, r);
    } G(10) {
    { u8x16 x = a.b; x = x - ((x >> 1) & 0x55); x = (x & 0x33) + ((x >> 2) & 0x33); x = (x + (x >> 4)) & 0x0F; r.b = x; h = fv(h, r); } r.b = __builtin_elementwise_bitreverse(a.b); h = fv(h, r);
    }
  }
  return h;
}
