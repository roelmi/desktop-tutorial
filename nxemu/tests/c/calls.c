#include "rt.h"
static uint64_t fib(uint32_t n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
static uint64_t gcd(uint64_t a, uint64_t b) { return b ? gcd(b, a % b) : a; }
static uint64_t ack(uint64_t m, uint64_t n) { return m == 0 ? n + 1 : (n == 0 ? ack(m - 1, 1) : ack(m - 1, ack(m, n - 1))); }
typedef uint64_t (*fn)(uint64_t);
static uint64_t f0(uint64_t x) { return x + 1; } static uint64_t f1(uint64_t x) { return x * 3; }
static uint64_t f2(uint64_t x) { return x ^ 0xABCDEF; } static uint64_t f3(uint64_t x) { return x >> 1; }
static fn table[4] = { f0, f1, f2, f3 };
static uint64_t sw(uint32_t k, uint64_t x) {
  switch (k) { case 0: return x + 7; case 1: return x * 11; case 2: return x - 5; case 3: return x << 3;
    case 4: return x ^ 0x55; case 5: return x | 0x100; case 6: return ~x; case 7: return x / 3; case 9: return x % 1000; default: return x; }
}
static uint64_t collatz(uint64_t n) { uint64_t steps = 0; while (n != 1) { n = (n & 1) ? 3 * n + 1 : n / 2; steps++; } return steps; }
uint64_t run(void) {
  uint64_t h = 5;
  h = mix(h, fib(22)); h = mix(h, ack(2, 3)); h = mix(h, ack(3, 3));
  for (uint64_t i = 1; i < 200; i++) h = mix(h, gcd(i * 7919, 104729 + i));
  uint64_t x = 1; for (int i = 0; i < 400; i++) { x = table[i & 3](x) + (uint64_t)i; h = mix(h, x); }
  for (uint32_t k = 0; k < 12; k++) for (uint64_t v = 1; v < 40; v += 7) h = mix(h, sw(k, v * 12345));
  for (uint64_t n = 1; n < 120; n++) h = mix(h, collatz(n));
  return h;
}
