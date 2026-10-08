// Mini runtime para las pruebas diferenciales.
//  - Nativo (-DNATIVE): imprime el resultado de run().
//  - ARM64 sin SO: _start fija la pila, llama a run() y termina con svc #7 (ExitProcess, x0 = resultado).
#include <stdint.h>
uint64_t run(void);
#ifdef NATIVE
#include <stdio.h>
int main(void) { printf("%llu\n", (unsigned long long)run()); return 0; }
#else
__attribute__((naked, section(".text.start"))) void _start(void) {
  __asm__ volatile("movz x9, #0x1000, lsl #16\n mov sp, x9\n bl run\n svc #7\n");
}
#endif
// Mezcla un valor en el checksum (FNV-1a de 64 bits sobre palabras).
static inline uint64_t mix(uint64_t h, uint64_t v) { return (h ^ v) * 0x100000001b3ULL + (h >> 29); }
// Generador pseudoaleatorio determinista (xorshift64*).
static inline uint64_t rng(uint64_t *s) { uint64_t x = *s; x ^= x >> 12; x ^= x << 25; x ^= x >> 27; *s = x; return x * 0x2545F4914F6CDD1DULL; }
