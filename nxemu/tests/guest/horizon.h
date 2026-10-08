// Envoltorios mínimos de las SVC de Horizon para los programas de prueba (sin libnx).
#pragma once
#include <stdint.h>
typedef uint32_t Result;
typedef uint32_t Handle;
#define CUR_THREAD 0xFFFF8000u
#define SVC_ARGS_0(n) register uint64_t x0 __asm__("x0"); __asm__ volatile("svc " #n : "=r"(x0) :: "memory");
static inline Result svcSetHeapSize(void **out, uint64_t size) {
  register uint64_t x0 __asm__("x0") = 0, x1 __asm__("x1") = size;
  __asm__ volatile("svc #0x01" : "+r"(x0), "+r"(x1) :: "memory");
  *out = (void *)x1; return (Result)x0;
}
static inline Result svcSetMemoryPermission(void *addr, uint64_t size, uint32_t perm) {
  register uint64_t x0 __asm__("x0") = (uint64_t)addr, x1 __asm__("x1") = size, x2 __asm__("x2") = perm;
  __asm__ volatile("svc #0x02" : "+r"(x0) : "r"(x1), "r"(x2) : "memory"); return (Result)x0;
}
static inline Result svcMapMemory(void *dst, void *src, uint64_t size) {
  register uint64_t x0 __asm__("x0") = (uint64_t)dst, x1 __asm__("x1") = (uint64_t)src, x2 __asm__("x2") = size;
  __asm__ volatile("svc #0x04" : "+r"(x0) : "r"(x1), "r"(x2) : "memory"); return (Result)x0;
}
static inline Result svcUnmapMemory(void *dst, void *src, uint64_t size) {
  register uint64_t x0 __asm__("x0") = (uint64_t)dst, x1 __asm__("x1") = (uint64_t)src, x2 __asm__("x2") = size;
  __asm__ volatile("svc #0x05" : "+r"(x0) : "r"(x1), "r"(x2) : "memory"); return (Result)x0;
}
typedef struct { uint64_t addr, size; uint32_t type, attr, perm, ipc, dev, pad; } MemoryInfo;
static inline Result svcQueryMemory(MemoryInfo *mi, uint32_t *pi, uint64_t addr) {
  register uint64_t x0 __asm__("x0") = (uint64_t)mi, x1 __asm__("x1") = 0, x2 __asm__("x2") = addr;
  __asm__ volatile("svc #0x06" : "+r"(x0), "+r"(x1) : "r"(x2) : "memory"); *pi = (uint32_t)x1; return (Result)x0;
}
static inline __attribute__((noreturn)) void svcExitProcess(uint64_t code) {
  register uint64_t x0 __asm__("x0") = code; __asm__ volatile("svc #0x07" :: "r"(x0) : "memory"); for (;;);
}
static inline Result svcCreateThread(Handle *out, void *entry, void *arg, void *stack_top, int32_t prio, int32_t core) {
  register uint64_t x0 __asm__("x0") = (uint64_t)entry, x1 __asm__("x1") = (uint64_t)arg, x2 __asm__("x2") = (uint64_t)stack_top,
                    x3 __asm__("x3") = (uint32_t)prio, x4 __asm__("x4") = (uint32_t)core;
  __asm__ volatile("svc #0x08" : "+r"(x0), "+r"(x1) : "r"(x2), "r"(x3), "r"(x4) : "memory");
  *out = (Handle)x1; return (Result)x0;
}
static inline Result svcStartThread(Handle h) {
  register uint64_t x0 __asm__("x0") = h; __asm__ volatile("svc #0x09" : "+r"(x0) :: "memory"); return (Result)x0;
}
static inline __attribute__((noreturn)) void svcExitThread(void) { __asm__ volatile("svc #0x0A" ::: "memory"); for (;;); }
static inline void svcSleepThread(int64_t ns) {
  register uint64_t x0 __asm__("x0") = (uint64_t)ns; __asm__ volatile("svc #0x0B" : "+r"(x0) :: "memory");
}
static inline Result svcGetThreadPriority(int32_t *out, Handle h) {
  register uint64_t x0 __asm__("x0"), x1 __asm__("x1") = h; __asm__ volatile("svc #0x0C" : "=r"(x0), "+r"(x1) :: "memory");
  *out = (int32_t)x1; return (Result)x0;
}
static inline Result svcCloseHandle(Handle h) {
  register uint64_t x0 __asm__("x0") = h; __asm__ volatile("svc #0x16" : "+r"(x0) :: "memory"); return (Result)x0;
}
static inline Result svcWaitSynchronization(int32_t *idx, const Handle *hs, int32_t n, int64_t timeout) {
  register uint64_t x0 __asm__("x0"), x1 __asm__("x1") = (uint64_t)hs, x2 __asm__("x2") = (uint32_t)n, x3 __asm__("x3") = (uint64_t)timeout;
  __asm__ volatile("svc #0x18" : "=r"(x0), "+r"(x1) : "r"(x2), "r"(x3) : "memory"); *idx = (int32_t)x1; return (Result)x0;
}
static inline Result svcArbitrateLock(Handle owner, uint32_t *addr, Handle self) {
  register uint64_t x0 __asm__("x0") = owner, x1 __asm__("x1") = (uint64_t)addr, x2 __asm__("x2") = self;
  __asm__ volatile("svc #0x1A" : "+r"(x0) : "r"(x1), "r"(x2) : "memory"); return (Result)x0;
}
static inline Result svcArbitrateUnlock(uint32_t *addr) {
  register uint64_t x0 __asm__("x0") = (uint64_t)addr; __asm__ volatile("svc #0x1B" : "+r"(x0) :: "memory"); return (Result)x0;
}
static inline Result svcWaitProcessWideKeyAtomic(uint32_t *mutex, uint32_t *key, uint32_t tag, int64_t timeout) {
  register uint64_t x0 __asm__("x0") = (uint64_t)mutex, x1 __asm__("x1") = (uint64_t)key, x2 __asm__("x2") = tag, x3 __asm__("x3") = (uint64_t)timeout;
  __asm__ volatile("svc #0x1C" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x3) : "memory"); return (Result)x0;
}
static inline void svcSignalProcessWideKey(uint32_t *key, int32_t count) {
  register uint64_t x0 __asm__("x0") = (uint64_t)key, x1 __asm__("x1") = (uint32_t)count; __asm__ volatile("svc #0x1D" : "+r"(x0) : "r"(x1) : "memory");
}
static inline uint64_t svcGetSystemTick(void) { register uint64_t x0 __asm__("x0"); __asm__ volatile("svc #0x1E" : "=r"(x0) :: "memory"); return x0; }
static inline Result svcConnectToNamedPort(Handle *out, const char *name) {
  register uint64_t x0 __asm__("x0"), x1 __asm__("x1") = (uint64_t)name; __asm__ volatile("svc #0x1F" : "=r"(x0), "+r"(x1) :: "memory");
  *out = (Handle)x1; return (Result)x0;
}
static inline Result svcSendSyncRequest(Handle h) {
  register uint64_t x0 __asm__("x0") = h; __asm__ volatile("svc #0x21" : "+r"(x0) :: "memory"); return (Result)x0;
}
static inline void svcOutputDebugString(const char *s, uint64_t n) {
  register uint64_t x0 __asm__("x0") = (uint64_t)s, x1 __asm__("x1") = n; __asm__ volatile("svc #0x27" : "+r"(x0) : "r"(x1) : "memory");
}
static inline Result svcGetInfo(uint64_t *out, uint32_t type, Handle h, uint64_t id) {
  register uint64_t x0 __asm__("x0"), x1 __asm__("x1") = type, x2 __asm__("x2") = h, x3 __asm__("x3") = id;
  __asm__ volatile("svc #0x29" : "=r"(x0), "+r"(x1) : "r"(x2), "r"(x3) : "memory"); *out = x1; return (Result)x0;
}
static inline Result svcCreateEvent(Handle *w, Handle *r) {
  register uint64_t x0 __asm__("x0"), x1 __asm__("x1"), x2 __asm__("x2"); __asm__ volatile("svc #0x45" : "=r"(x0), "=r"(x1), "=r"(x2) :: "memory");
  *w = (Handle)x1; *r = (Handle)x2; return (Result)x0;
}
static inline Result svcSignalEvent(Handle h) {
  register uint64_t x0 __asm__("x0") = h; __asm__ volatile("svc #0x11" : "+r"(x0) :: "memory"); return (Result)x0;
}
static inline Result svcResetSignal(Handle h) {
  register uint64_t x0 __asm__("x0") = h; __asm__ volatile("svc #0x17" : "+r"(x0) :: "memory"); return (Result)x0;
}

// ---- IPC (formato HIPC/CMIF) --------------------------------------------------------------------------
static inline uint32_t *tls_buf(void) { uint64_t t; __asm__ volatile("mrs %0, tpidr_el0" : "=r"(t)); return (uint32_t *)t; }
// Envía una petición sin descriptores con `payload` (n palabras de 32 bits tras el id de comando).
static inline Result ipcRequest(Handle session, uint32_t type, uint32_t cmd, const uint32_t *payload, int nwords, int send_pid) {
  uint32_t *c = tls_buf(); int i = 0;
  c[i++] = type; int raw_words = 4 + nwords;
  c[i++] = raw_words | (send_pid ? (1u << 31) : 0u);
  if (send_pid) { c[i++] = 1; c[i++] = 0; c[i++] = 0; }
  while (i % 4) c[i++] = 0;                         // datos crudos alineados a 16 bytes
  c[i++] = 0x49434653; c[i++] = 0; c[i++] = cmd; c[i++] = 0;
  for (int k = 0; k < nwords; k++) c[i++] = payload[k];
  return svcSendSyncRequest(session);
}
// Lee la respuesta: devuelve el código de resultado y deja punteros a la carga útil y al primer handle movido.
static inline uint32_t ipcResponse(const uint32_t **payload, Handle *moved) {
  uint32_t *c = tls_buf(); uint32_t w1 = c[1]; int i = 2; Handle mv = 0;
  if (w1 >> 31) { uint32_t hd = c[i++]; int ncopy = (hd >> 1) & 15, nmove = (hd >> 5) & 15; i += ncopy; if (nmove) mv = c[i]; i += nmove; }
  while (i % 4) i++;
  if (moved) *moved = mv;
  if (payload) *payload = &c[i + 4];
  return c[i + 2];
}
