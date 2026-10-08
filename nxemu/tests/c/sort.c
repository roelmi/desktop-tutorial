#include "rt.h"
#define N 300
static uint64_t arr[N], tmp[N];
static int32_t small[N];
static void insertion(uint64_t *a, int n) { for (int i = 1; i < n; i++) { uint64_t k = a[i]; int j = i - 1; while (j >= 0 && a[j] > k) { a[j + 1] = a[j]; j--; } a[j + 1] = k; } }
static void quick(uint64_t *a, int lo, int hi) {
  while (lo < hi) {
    uint64_t p = a[(lo + hi) / 2]; int i = lo, j = hi;
    while (i <= j) { while (a[i] < p) i++; while (a[j] > p) j--; if (i <= j) { uint64_t t = a[i]; a[i] = a[j]; a[j] = t; i++; j--; } }
    if (j - lo < hi - i) { quick(a, lo, j); lo = i; } else { quick(a, i, hi); hi = j; }
  }
}
static void msort(uint64_t *a, uint64_t *t, int n) {
  if (n < 2) return; int m = n / 2; msort(a, t, m); msort(a + m, t, n - m);
  int i = 0, j = m, k = 0; while (i < m && j < n) t[k++] = a[i] <= a[j] ? a[i++] : a[j++];
  while (i < m) t[k++] = a[i++]; while (j < n) t[k++] = a[j++];
  for (i = 0; i < n; i++) a[i] = t[i];
}
uint64_t run(void) {
  uint64_t h = 3, s = 12345;
  for (int round = 0; round < 3; round++) {
    for (int i = 0; i < N; i++) arr[i] = rng(&s);
    if (round == 0) insertion(arr, N); else if (round == 1) quick(arr, 0, N - 1); else msort(arr, tmp, N);
    for (int i = 0; i < N; i++) h = mix(h, arr[i] * (uint64_t)(i + 1));
    for (int i = 1; i < N; i++) h = mix(h, arr[i - 1] <= arr[i]);
  }
  for (int i = 0; i < N; i++) small[i] = (int32_t)rng(&s);
  for (int i = 0; i < N; i++) for (int j = 0; j + 1 < N - i; j++) if (small[j] > small[j + 1]) { int32_t t = small[j]; small[j] = small[j + 1]; small[j + 1] = t; }
  for (int i = 0; i < N; i++) h = mix(h, (uint64_t)(int64_t)small[i]);
  return h;
}
