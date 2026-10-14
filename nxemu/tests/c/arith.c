#include "rt.h"
uint64_t run(void) {
  uint64_t h = 1, s = 88172645463325252ULL;
  for (int i = 0; i < 2000; i++) {
    uint64_t a = rng(&s), b = rng(&s) | 1;
    int64_t sa = (int64_t)a, sb = (int64_t)b;
    uint32_t wa = (uint32_t)a, wb = (uint32_t)b | 1u;
    int32_t ia = (int32_t)a, ib = (int32_t)b | 1;
    h = mix(h, a + b); h = mix(h, a - b); h = mix(h, a * b);
    h = mix(h, a / b); h = mix(h, a % b);
    if (sb != -1) { h = mix(h, (uint64_t)(sa / sb)); h = mix(h, (uint64_t)(sa % sb)); }
    h = mix(h, wa / wb); h = mix(h, wa % wb); h = mix(h, (uint32_t)(wa * wb));
    if (ib != -1) { h = mix(h, (uint32_t)(ia / ib)); h = mix(h, (uint32_t)(ia % ib)); }
    h = mix(h, a << (b & 63)); h = mix(h, a >> (b & 63)); h = mix(h, (uint64_t)(sa >> (b & 63)));
    h = mix(h, (uint32_t)(wa << (wb & 31))); h = mix(h, wa >> (wb & 31)); h = mix(h, (uint32_t)(ia >> (wb & 31)));
    h = mix(h, (uint64_t)(int64_t)(int8_t)a); h = mix(h, (uint64_t)(int64_t)(int16_t)a); h = mix(h, (uint64_t)(int64_t)(int32_t)a);
    h = mix(h, (uint8_t)a); h = mix(h, (uint16_t)a);
    h = mix(h, a < b); h = mix(h, sa < sb); h = mix(h, a == b); h = mix(h, a > b ? a : b); h = mix(h, sa < sb ? sa : sb);
    h = mix(h, ~a); h = mix(h, a & b); h = mix(h, a | b); h = mix(h, a ^ b); h = mix(h, a & ~b);
    h = mix(h, (uint64_t)-(int64_t)a);
  }
  return h;
}
