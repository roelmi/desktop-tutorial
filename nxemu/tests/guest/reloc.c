// Punteros a funciones y a cadenas en datos inicializados: solo funcionan si el cargador aplica las reubicaciones.
#include "horizon.h"
#define CHECK(c) do { if (!(c)) svcExitProcess(__LINE__); } while (0)
static uint64_t f_add(uint64_t a) { return a + 10; }
static uint64_t f_mul(uint64_t a) { return a * 3; }
static uint64_t f_neg(uint64_t a) { return ~a; }
typedef uint64_t (*fn)(uint64_t);
static const fn rotulos[] = { f_add, f_mul, f_neg };
static fn mutable_tbl[] = { f_mul, f_add };
static const char *const nombres[] = { "uno", "dos", "tres" };
extern char __bss_start[];
int g_dato = 7; int *g_ptr = &g_dato;      // puntero a un global inicializado (RELATIVE en .data)
void _start(void) {
  CHECK(rotulos[0](1) == 11 && rotulos[1](5) == 15 && rotulos[2](0) == ~0ULL);
  CHECK(mutable_tbl[0](4) == 12 && mutable_tbl[1](4) == 14);
  CHECK(nombres[1][0] == 'd' && nombres[2][2] == 'e' && nombres[0][3] == 0);
  CHECK(*g_ptr == 7); *g_ptr = 9; CHECK(g_dato == 9);
  // la dirección de las funciones debe estar dentro de la imagen cargada en 0x08000000
  CHECK((uint64_t)rotulos[0] >= 0x08000000 && (uint64_t)rotulos[0] < 0x08100000);
  CHECK((uint64_t)nombres[0] >= 0x08000000 && (uint64_t)nombres[0] < 0x08100000);
  svcExitProcess(0);
}
