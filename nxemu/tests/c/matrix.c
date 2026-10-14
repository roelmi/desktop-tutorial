#include "rt.h"
#define D 8
typedef struct { int32_t x, y; uint64_t tag; } P;
static int64_t A[D][D], B[D][D], C[D][D];
static P pts[64];
static P swapP(P p) { P q = { p.y, p.x, p.tag * 3 }; return q; }
uint64_t run(void) {
  uint64_t h = 17, s = 777;
  for (int i = 0; i < D; i++) for (int j = 0; j < D; j++) { A[i][j] = (int64_t)(rng(&s) % 2001) - 1000; B[i][j] = (int64_t)(rng(&s) % 2001) - 1000; }
  for (int i = 0; i < D; i++) for (int j = 0; j < D; j++) { int64_t t = 0; for (int k = 0; k < D; k++) t += A[i][k] * B[k][j]; C[i][j] = t; }
  for (int i = 0; i < D; i++) for (int j = 0; j < D; j++) h = mix(h, (uint64_t)C[i][j]);
  for (int i = 0; i < 64; i++) { pts[i].x = (int32_t)rng(&s); pts[i].y = (int32_t)rng(&s); pts[i].tag = rng(&s); }
  for (int i = 0; i < 64; i++) { P q = swapP(pts[i]); pts[i] = q; h = mix(h, (uint64_t)(int64_t)q.x); h = mix(h, (uint64_t)(int64_t)q.y); h = mix(h, q.tag); }
  return h;
}
