// Bucles que clang vectoriza solo (NEON generado por el compilador, incluidas cargas intercaladas).
#include "rt.h"
#define N 256
static float fa[N], fb[N], fc[N]; static double da[N], db[N];
static int32_t ia[N], ib[N], ic[N]; static int16_t sa[N], sb[N]; static uint8_t ba[N], bb[N], bc[N];
static uint32_t ua[N]; static int64_t la[N];
static float xyz[3 * N], out3[3 * N]; static uint16_t pix[2 * N];
uint64_t run(void) {
  uint64_t h = 41, s = 6061;
  for (int i = 0; i < N; i++) {
    fa[i] = (float)(int32_t)(rng(&s) >> 44) / 64.0f; fb[i] = (float)(int32_t)(rng(&s) >> 44) / 32.0f + 0.5f;
    da[i] = (double)(int64_t)(rng(&s) >> 40) / 256.0; db[i] = (double)(int64_t)(rng(&s) >> 40) / 128.0 + 1.0;
    ia[i] = (int32_t)rng(&s); ib[i] = (int32_t)rng(&s); sa[i] = (int16_t)rng(&s); sb[i] = (int16_t)rng(&s);
    ba[i] = (uint8_t)rng(&s); bb[i] = (uint8_t)rng(&s); ua[i] = (uint32_t)rng(&s); la[i] = (int64_t)rng(&s);
  }
  for (int i = 0; i < 3 * N; i++) xyz[i] = (float)(int32_t)(rng(&s) >> 44) / 16.0f;
  for (int i = 0; i < 2 * N; i++) pix[i] = (uint16_t)rng(&s);
  // saxpy y variantes
  for (int i = 0; i < N; i++) fc[i] = 2.5f * fa[i] + fb[i]; for (int i = 0; i < N; i++) h = mix(h, *(uint32_t *)&fc[i]);
  for (int i = 0; i < N; i++) fc[i] = fa[i] * fb[i] - fa[i] / fb[i]; for (int i = 0; i < N; i++) h = mix(h, *(uint32_t *)&fc[i]);
  for (int i = 0; i < N; i++) db[i] = da[i] * db[i] + da[i]; for (int i = 0; i < N; i++) h = mix(h, *(uint64_t *)&db[i]);
  // reducciones
  int32_t dot = 0; for (int i = 0; i < N; i++) dot += ia[i] * ib[i]; h = mix(h, (uint32_t)dot);
  int64_t wide = 0; for (int i = 0; i < N; i++) wide += (int64_t)sa[i] * sb[i]; h = mix(h, (uint64_t)wide);
  uint32_t bs = 0; for (int i = 0; i < N; i++) bs += ba[i]; h = mix(h, bs);
  int32_t mx = ia[0], mn = ia[0]; for (int i = 1; i < N; i++) { mx = ia[i] > mx ? ia[i] : mx; mn = ia[i] < mn ? ia[i] : mn; } h = mix(h, (uint32_t)mx); h = mix(h, (uint32_t)mn);
  uint32_t sad = 0; for (int i = 0; i < N; i++) sad += ba[i] > bb[i] ? ba[i] - bb[i] : bb[i] - ba[i]; h = mix(h, sad);
  float fs = 0; for (int i = 0; i < 8; i++) fs += fa[i]; h = mix(h, *(uint32_t *)&fs);
  // elemento a elemento, conversiones y anchos mixtos
  for (int i = 0; i < N; i++) ic[i] = ia[i] + (ib[i] >> 3) - (ia[i] & ib[i]); for (int i = 0; i < N; i++) h = mix(h, (uint32_t)ic[i]);
  for (int i = 0; i < N; i++) fc[i] = (float)ba[i] * 0.5f; for (int i = 0; i < N; i += 7) h = mix(h, *(uint32_t *)&fc[i]);
  for (int i = 0; i < N; i++) ic[i] = (int32_t)(fa[i] * 3.0f); for (int i = 0; i < N; i++) h = mix(h, (uint32_t)ic[i]);
  for (int i = 0; i < N; i++) bc[i] = (uint8_t)(sa[i] >> 4); for (int i = 0; i < N; i++) h = mix(h, bc[i]);
  for (int i = 0; i < N; i++) ia[i] = sa[i] * sb[i] + ia[i]; for (int i = 0; i < N; i++) h = mix(h, (uint32_t)ia[i]);
  for (int i = 0; i < N; i++) { int32_t v = ib[i]; ic[i] = v < -1000 ? -1000 : (v > 1000 ? 1000 : v); } for (int i = 0; i < N; i++) h = mix(h, (uint32_t)ic[i]);
  for (int i = 0; i < N; i++) ua[i] = __builtin_bswap32(ua[i]); for (int i = 0; i < N; i++) h = mix(h, ua[i]);
  for (int i = 0; i < N; i++) la[i] = la[i] * 3 + (la[i] >> 5); for (int i = 0; i < N; i++) h = mix(h, (uint64_t)la[i]);
  for (int i = 0; i < N; i++) bc[i] = ba[i] ^ (uint8_t)(bb[i] << 1); for (int i = 0; i < N; i++) h = mix(h, bc[i]);
  int cnt = 0; for (int i = 0; i < N; i++) cnt += ib[i] > 0; h = mix(h, (uint32_t)cnt);
  // estructuras intercaladas (LD3/ST3) y pares (LD2/ST2)
  for (int i = 0; i < N; i++) { out3[3 * i] = xyz[3 * i] * 2.0f; out3[3 * i + 1] = xyz[3 * i + 1] + 1.0f; out3[3 * i + 2] = xyz[3 * i + 2] - xyz[3 * i]; }
  for (int i = 0; i < 3 * N; i += 5) h = mix(h, *(uint32_t *)&out3[i]);
  uint32_t lo = 0, hi = 0; for (int i = 0; i < N; i++) { lo += pix[2 * i]; hi += pix[2 * i + 1]; } h = mix(h, lo); h = mix(h, hi);
  return h;
}
