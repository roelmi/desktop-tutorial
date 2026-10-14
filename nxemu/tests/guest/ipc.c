#include "horizon.h"
#define CHECK(c) do { if (!(c)) svcExitProcess(__LINE__); } while (0)
static const char dbg[] = "hola desde el invitado\n";
void _start(void) {
  Handle sm, svc; Handle dummy;
  CHECK(svcConnectToNamedPort(&dummy, "nope") == 0xF201);
  CHECK(svcConnectToNamedPort(&sm, "sm:") == 0);
  // sm: Initialize (cmd 0), pidiendo envío de PID
  CHECK(ipcRequest(sm, 4, 0, 0, 0, 1) == 0); CHECK(ipcResponse(0, 0) == 0);
  // GetService("set") -> sesión nueva
  uint32_t name[2] = { 0x00746573, 0 };              // "set\0\0\0\0\0"
  CHECK(ipcRequest(sm, 4, 1, name, 2, 0) == 0);
  Handle set; CHECK(ipcResponse(0, &set) == 0); CHECK(set != 0 && set != sm);
  // set: GetLanguageCode (cmd 0) -> u64 "en-US"
  CHECK(ipcRequest(set, 4, 0, 0, 0, 0) == 0);
  const uint32_t *p; CHECK(ipcResponse(&p, 0) == 0);
  CHECK(p[0] == 0x552D6E65 && p[1] == 0x53);                                  // "en-US"
  // comando desconocido -> error de servicio
  CHECK(ipcRequest(set, 4, 99, 0, 0, 0) == 0); CHECK(ipcResponse(0, 0) != 0);
  // servicio no registrado
  uint32_t bad[2] = { 0x6C6C756E, 0 };               // "null"
  CHECK(ipcRequest(sm, 4, 1, bad, 2, 0) == 0); CHECK(ipcResponse(0, 0) != 0);
  // control: QueryPointerBufferSize (3) y CloneCurrentObject (2)
  CHECK(ipcRequest(set, 5, 3, 0, 0, 0) == 0); CHECK(ipcResponse(&p, 0) == 0 && p[0] == 0x8000);
  CHECK(ipcRequest(set, 5, 2, 0, 0, 0) == 0); Handle clone; CHECK(ipcResponse(0, &clone) == 0 && clone != 0);
  CHECK(ipcRequest(clone, 4, 0, 0, 0, 0) == 0); CHECK(ipcResponse(0, 0) == 0);
  // apm: OpenSession -> SetPerformanceConfiguration / GetPerformanceConfiguration
  Handle apm, ses; CHECK(svcConnectToNamedPort(&apm, "sm:") == 0);
  CHECK(ipcRequest(apm, 4, 0, 0, 0, 1) == 0); CHECK(ipcResponse(0, 0) == 0);
  uint32_t an[2] = { 0x006D7061, 0 };
  CHECK(ipcRequest(apm, 4, 1, an, 2, 0) == 0); Handle apmsvc; CHECK(ipcResponse(0, &apmsvc) == 0);
  CHECK(ipcRequest(apmsvc, 4, 0, 0, 0, 0) == 0); CHECK(ipcResponse(0, &ses) == 0 && ses != 0);
  uint32_t cfg[2] = { 1, 0x20004 }; CHECK(ipcRequest(ses, 4, 0, cfg, 2, 0) == 0); CHECK(ipcResponse(0, 0) == 0);
  uint32_t m1 = 1; CHECK(ipcRequest(ses, 4, 1, &m1, 1, 0) == 0); CHECK(ipcResponse(&p, 0) == 0 && p[0] == 0x20004);
  CHECK(svcCloseHandle(clone) == 0); CHECK(svcCloseHandle(clone) == 0xE401);
  CHECK(ipcRequest(clone, 4, 0, 0, 0, 0) == 0xE401);                           // handle ya cerrado
  svcOutputDebugString(dbg, sizeof(dbg) - 1);
  svcExitProcess(0);
}
