#include "horizon.h"
#define CHECK(c) do { if (!(c)) svcExitProcess(__LINE__); } while (0)
extern char __bss_start[]; static char scratch[0x2000] __attribute__((aligned(0x1000)));
void _start(void) {
  void *heap = 0;
  Result r = svcSetHeapSize(&heap, 0x300000); CHECK(r == 0xCA01);   // no es múltiplo de 0x200000
  CHECK(svcSetHeapSize(&heap, 0x400000) == 0); CHECK(heap == (void *)0x20000000);
  volatile uint64_t *h = heap; h[0] = 0x1122334455667788ULL; h[0x3FFFF0 / 8] = 42; CHECK(h[0] == 0x1122334455667788ULL);
  uint64_t v; CHECK(svcGetInfo(&v, 4, 0xFFFF8001, 0) == 0 && v == 0x20000000);
  MemoryInfo mi; uint32_t pi;
  CHECK(svcQueryMemory(&mi, &pi, 0x20001000) == 0); CHECK(mi.addr == 0x20000000 && mi.size == 0x400000 && mi.type == 5 && mi.perm == 3);
  CHECK(svcQueryMemory(&mi, &pi, 0x30000000) == 0); CHECK(mi.type == 0 && mi.perm == 0);        // sin mapear
  // permisos: pasar a sólo lectura y comprobar
  CHECK(svcSetMemoryPermission((void *)0x20000000, 0x1000, 1) == 0);
  CHECK(svcQueryMemory(&mi, &pi, 0x20000000) == 0 && mi.perm == 1 && mi.size == 0x1000);
  CHECK(svcSetMemoryPermission((void *)0x20000001, 0x1000, 1) == 0xCC01);   // dirección desalineada
  CHECK(svcSetMemoryPermission((void *)0x30000000, 0x1000, 1) == 0xD401);   // sin mapear
  CHECK(svcSetMemoryPermission((void *)0x20000000, 0x1000, 3) == 0);
  // MapMemory / UnmapMemory: vista alias de una página del heap
  h[0x1000 / 8] = 0xABCD; CHECK(svcMapMemory((void *)0x40000000, (void *)0x20001000, 0x1000) == 0);
  volatile uint64_t *alias = (volatile uint64_t *)0x40000000; CHECK(alias[0] == 0xABCD); alias[0] = 0xDCBA;
  CHECK(svcQueryMemory(&mi, &pi, 0x20001000) == 0 && mi.perm == 0);         // el origen queda inaccesible
  CHECK(svcUnmapMemory((void *)0x40000000, (void *)0x20001000, 0x1000) == 0); CHECK(h[0x1000 / 8] == 0xDCBA);
  CHECK(svcMapMemory((void *)0x10000000, (void *)0x20001000, 0x1000) == 0xD401);  // destino fuera de la región alias
  // reducir el heap libera las páginas
  CHECK(svcSetHeapSize(&heap, 0x200000) == 0); CHECK(svcQueryMemory(&mi, &pi, 0x20200000) == 0 && mi.type == 0);
  // imagen: .text es RX (tipo código estático), la pila es normal
  CHECK(svcQueryMemory(&mi, &pi, 0x08000000) == 0 && mi.perm == 5 && mi.type == 3);
  scratch[5] = 7; CHECK(svcQueryMemory(&mi, &pi, (uint64_t)scratch) == 0 && mi.perm == 3 && mi.type == 4);  // .bss: RW
  svcExitProcess(0);
}
