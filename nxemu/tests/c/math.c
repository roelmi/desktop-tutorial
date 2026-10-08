#include "rt.h"
typedef unsigned __int128 u128; typedef __int128 i128;
static uint64_t isqrt(uint64_t n) { uint64_t x = n, y = (x + 1) / 2; while (y < x) { x = y; y = (x + n / x) / 2; } return x; }
static uint64_t mulmod(uint64_t a, uint64_t b, uint64_t m) { uint64_t r = 0; a %= m; while (b) { if (b & 1) { r = (r >= m - a) ? r - (m - a) : r + a; } a = (a >= m - a) ? a - (m - a) : a + a; b >>= 1; } return r; }
static uint64_t modpow(uint64_t b, uint64_t e, uint64_t m) { uint64_t r = 1; b %= m; while (e) { if (e & 1) r = mulmod(r, b, m); b = mulmod(b, b, m); e >>= 1; } return r; }
uint64_t run(void) {
  uint64_t h = 13, s = 424242;
  for (int i = 0; i < 400; i++) {
    uint64_t a = rng(&s), b = rng(&s);
    u128 p = (u128)a * b; i128 q = (i128)(int64_t)a * (int64_t)b;
    h = mix(h, (uint64_t)(p >> 64)); h = mix(h, (uint64_t)p); h = mix(h, (uint64_t)((u128)q >> 64));
    h = mix(h, a / 10); h = mix(h, a / 1000003); h = mix(h, a % 97); h = mix(h, (uint64_t)((int64_t)a / 7)); h = mix(h, (uint64_t)((int64_t)a % 13));
    h = mix(h, (uint32_t)a / 10u); h = mix(h, (uint32_t)((int32_t)a / 9));
    h = mix(h, isqrt(a >> 2));
    h = mix(h, a + b < a); h = mix(h, (uint64_t)(int64_t)((int64_t)a - (int64_t)b));
    uint64_t lo = a * b, hi = (uint64_t)(((u128)a * b) >> 64); uint64_t carry = lo + a < lo; h = mix(h, hi + carry);
  }
  h = mix(h, modpow(2, 1000003, 1000000007ULL)); h = mix(h, modpow(3, 123456789, 998244353ULL)); h = mix(h, modpow(12345, 6789, 0xFFFFFFFFFFFFFFC5ULL));
  return h;
}
