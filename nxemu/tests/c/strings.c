#include "rt.h"
static uint64_t slen(const char *s) { uint64_t n = 0; while (s[n]) n++; return n; }
static int scmp(const char *a, const char *b) { while (*a && *a == *b) { a++; b++; } return (int)(unsigned char)*a - (int)(unsigned char)*b; }
static void mcopy(void *d, const void *s, uint64_t n) { unsigned char *dd = d; const unsigned char *ss = s; for (uint64_t i = 0; i < n; i++) dd[i] = ss[i]; }
static void mset(void *d, int c, uint64_t n) { unsigned char *dd = d; for (uint64_t i = 0; i < n; i++) dd[i] = (unsigned char)c; }
static uint32_t crc_table[256];
static uint32_t crc32(const unsigned char *p, uint64_t n) { uint32_t c = 0xFFFFFFFFu; for (uint64_t i = 0; i < n; i++) c = crc_table[(c ^ p[i]) & 0xFF] ^ (c >> 8); return ~c; }
static uint64_t fnv(const char *s) { uint64_t h = 1469598103934665603ULL; while (*s) { h ^= (unsigned char)*s++; h *= 1099511628211ULL; } return h; }
static const char *words[] = { "switch", "emulator", "aarch64", "nexus", "zelda", "echoes of wisdom", "link's awakening", "", "A", "zzz" };
static unsigned char buf[512], buf2[512];
static int8_t sb[64]; static int16_t sh[64]; static uint16_t uh[64];
uint64_t run(void) {
  uint64_t h = 11;
  for (uint32_t i = 0; i < 256; i++) { uint32_t c = i; for (int k = 0; k < 8; k++) c = (c & 1) ? 0xEDB88320u ^ (c >> 1) : c >> 1; crc_table[i] = c; }
  for (int i = 0; i < 10; i++) { h = mix(h, slen(words[i])); h = mix(h, fnv(words[i])); for (int j = 0; j < 10; j++) h = mix(h, (uint64_t)(int64_t)scmp(words[i], words[j])); }
  uint64_t s = 99; for (int i = 0; i < 512; i++) buf[i] = (unsigned char)rng(&s);
  h = mix(h, crc32(buf, 512)); mcopy(buf2, buf + 3, 300); h = mix(h, crc32(buf2, 300)); mset(buf2 + 10, 0xAB, 100); h = mix(h, crc32(buf2, 300));
  for (int i = 0; i < 64; i++) { sb[i] = (int8_t)(i * 37); sh[i] = (int16_t)(i * 1237 - 20000); uh[i] = (uint16_t)(i * 4001); }
  for (int i = 0; i < 64; i++) { h = mix(h, (uint64_t)(int64_t)sb[i]); h = mix(h, (uint64_t)(int64_t)sh[i]); h = mix(h, uh[i]); }
  return h;
}
