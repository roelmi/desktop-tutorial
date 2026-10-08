// Operaciones atómicas (LSE: LDADD, SWP, CAS…) en todos los tamaños y órdenes de memoria.
#include "rt.h"
static uint8_t a8; static uint16_t a16; static uint32_t a32; static uint64_t a64;
#define OPS(T, V, h, s)                                                                                  \
  do {                                                                                                  \
    T x = (T)rng(s), y = (T)rng(s), z = (T)rng(s);                                                       \
    __atomic_store_n(&V, x, __ATOMIC_RELAXED);                                                          \
    h = mix(h, (uint64_t)__atomic_fetch_add(&V, y, __ATOMIC_SEQ_CST)); h = mix(h, (uint64_t)V);          \
    h = mix(h, (uint64_t)__atomic_fetch_sub(&V, z, __ATOMIC_ACQUIRE)); h = mix(h, (uint64_t)V);          \
    h = mix(h, (uint64_t)__atomic_fetch_and(&V, y, __ATOMIC_RELEASE)); h = mix(h, (uint64_t)V);          \
    h = mix(h, (uint64_t)__atomic_fetch_or(&V, z, __ATOMIC_ACQ_REL)); h = mix(h, (uint64_t)V);           \
    h = mix(h, (uint64_t)__atomic_fetch_xor(&V, x, __ATOMIC_RELAXED)); h = mix(h, (uint64_t)V);          \
    h = mix(h, (uint64_t)__atomic_exchange_n(&V, y, __ATOMIC_SEQ_CST)); h = mix(h, (uint64_t)V);         \
    { T e = y; int ok = __atomic_compare_exchange_n(&V, &e, z, 0, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED);   \
      h = mix(h, (uint64_t)ok); h = mix(h, (uint64_t)e); h = mix(h, (uint64_t)V); }                      \
    { T e = (T)(y ^ 1); int ok = __atomic_compare_exchange_n(&V, &e, x, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED); \
      h = mix(h, (uint64_t)ok); h = mix(h, (uint64_t)e); h = mix(h, (uint64_t)V); }                      \
    h = mix(h, (uint64_t)__atomic_load_n(&V, __ATOMIC_ACQUIRE));                                         \
    __atomic_store_n(&V, z, __ATOMIC_RELEASE); h = mix(h, (uint64_t)V);                                  \
    h = mix(h, (uint64_t)__atomic_add_fetch(&V, x, __ATOMIC_RELAXED));                                   \
  } while (0)
uint64_t run(void) {
  uint64_t h = 43, s = 8080;
  for (int i = 0; i < 60; i++) { OPS(uint8_t, a8, h, &s); OPS(uint16_t, a16, h, &s); OPS(uint32_t, a32, h, &s); OPS(uint64_t, a64, h, &s); }
  return h;
}
