#include "rt.h"
static uint32_t popc(uint64_t x) { uint32_t n = 0; while (x) { x &= x - 1; n++; } return n; }
static uint64_t rotl(uint64_t x, unsigned r) { return (x << (r & 63)) | (x >> ((64 - r) & 63)); }
static uint32_t rotr32(uint32_t x, unsigned r) { return (x >> (r & 31)) | (x << ((32 - r) & 31)); }
uint64_t run(void) {
  uint64_t h = 7, s = 0x9E3779B97F4A7C15ULL;
  for (int i = 0; i < 1500; i++) {
    uint64_t a = rng(&s) >> (i % 40);
    uint32_t w = (uint32_t)(rng(&s) >> (i % 20));
    h = mix(h, popc(a));
    if (a) { h = mix(h, __builtin_clzll(a)); h = mix(h, __builtin_ctzll(a)); }
    if (w) { h = mix(h, __builtin_clz(w)); h = mix(h, __builtin_ctz(w)); }
    h = mix(h, __builtin_bswap64(a)); h = mix(h, __builtin_bswap32(w)); h = mix(h, __builtin_bswap16((uint16_t)w));
    h = mix(h, rotl(a, i)); h = mix(h, rotr32(w, i));
    h = mix(h, (a >> 13) & 0x7FF);               // extracción de campo (UBFX)
    h = mix(h, (uint64_t)((int64_t)(a << 20) >> 52)); // extracción con signo (SBFX)
    h = mix(h, (a & ~(0xFFFULL << 8)) | ((w & 0xFFFULL) << 8)); // inserción de campo (BFI)
    h = mix(h, __builtin_clrsb(w)); h = mix(h, __builtin_clrsbll((long long)a));
    h = mix(h, (uint64_t)__builtin_parityll(a));
  }
  return h;
}
