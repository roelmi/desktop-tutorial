#include "horizon.h"
#define CHECK(c) do { if (!(c)) svcExitProcess(__LINE__); } while (0)
static uint32_t mutex;        // 0 = libre; si no, handle del dueño | 0x40000000 si hay esperando
static uint32_t cv_key; static uint32_t flag;
static volatile uint64_t counter; static volatile int phase;
static Handle main_thread;
static void lock(Handle self) {
  for (;;) {
    uint32_t v = __atomic_load_n(&mutex, __ATOMIC_ACQUIRE);
    if (v == 0) { uint32_t z = 0; if (__atomic_compare_exchange_n(&mutex, &z, self, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) return; continue; }
    if (!(v & 0x40000000)) { if (!__atomic_compare_exchange_n(&mutex, &v, v | 0x40000000, 0, __ATOMIC_RELAXED, __ATOMIC_RELAXED)) continue; v |= 0x40000000; }
    svcArbitrateLock(v & ~0x40000000u, &mutex, self);
    if ((__atomic_load_n(&mutex, __ATOMIC_ACQUIRE) & ~0x40000000u) == self) return;
  }
}
static void unlock(Handle self) {
  uint32_t exp = self; if (__atomic_compare_exchange_n(&mutex, &exp, 0, 0, __ATOMIC_RELEASE, __ATOMIC_RELAXED)) return;
  svcArbitrateUnlock(&mutex);
}
static uint8_t stacks[4][0x4000] __attribute__((aligned(16)));
static void worker(void *arg) {
  uint64_t id = (uint64_t)arg; extern Handle self_handles[4]; Handle self = self_handles[id];
  for (int i = 0; i < 100; i++) { lock(self); uint64_t c = counter; if (i % 17 == 0) svcSleepThread(1000); counter = c + 1; unlock(self); }
  svcExitThread();
}
Handle self_handles[4];
static void waiter(void *arg) {
  Handle self = self_handles[3];
  lock(self);
  while (!flag) { svcWaitProcessWideKeyAtomic(&mutex, &cv_key, self, -1); if (__atomic_load_n(&mutex, __ATOMIC_ACQUIRE) != self) { phase = -1; svcExitThread(); } }
  phase = 2; unlock(self); svcExitThread();
}
void _start(void) {
  Handle t[4];
  // 3 hilos que incrementan un contador bajo un mutex
  for (int i = 0; i < 3; i++) { CHECK(svcCreateThread(&t[i], worker, (void *)(uint64_t)i, &stacks[i][0x4000], 44, 0) == 0); self_handles[i] = t[i]; }
  for (int i = 0; i < 3; i++) CHECK(svcStartThread(t[i]) == 0);
  int32_t idx;
  for (int i = 0; i < 3; i++) { CHECK(svcWaitSynchronization(&idx, &t[i], 1, -1) == 0 && idx == 0); }
  CHECK(counter == 300);
  CHECK(mutex == 0);
  // variable de condición: un hilo espera hasta que main señale
  Handle w; CHECK(svcCreateThread(&w, waiter, 0, &stacks[3][0x4000], 40, 0) == 0); self_handles[3] = w;
  CHECK(svcStartThread(w) == 0);
  svcSleepThread(1000000);                      // el hilo (prioridad más alta) ya debe estar esperando
  lock(0xCAFE);                                 // main usa un handle cualquiera como propietario
  flag = 1;
  unlock(0xCAFE);
  svcSignalProcessWideKey(&cv_key, 1);
  CHECK(svcWaitSynchronization(&idx, &w, 1, 2000000000) == 0);
  CHECK(phase == 2);
  // WaitSynchronization con tiempo agotado y con handle inválido
  Handle ev_w, ev_r; CHECK(svcCreateEvent(&ev_w, &ev_r) == 0);
  CHECK(svcWaitSynchronization(&idx, &ev_r, 1, 0) == 0xEA01);
  CHECK(svcWaitSynchronization(&idx, &ev_r, 1, 5000000) == 0xEA01);
  CHECK(svcSignalEvent(ev_w) == 0); CHECK(svcWaitSynchronization(&idx, &ev_r, 1, 0) == 0 && idx == 0);
  CHECK(svcResetSignal(ev_r) == 0); CHECK(svcResetSignal(ev_r) == 0xFA01);
  Handle bad = 0x1234; CHECK(svcWaitSynchronization(&idx, &bad, 1, 0) == 0xE401);
  // tiempo virtual: dormir avanza el reloj
  uint64_t a = svcGetSystemTick(); svcSleepThread(10000000); uint64_t b = svcGetSystemTick(); CHECK(b - a >= 192000);
  int32_t prio; CHECK(svcGetThreadPriority(&prio, CUR_THREAD) == 0 && prio == 44);
  svcExitProcess(0);
}
