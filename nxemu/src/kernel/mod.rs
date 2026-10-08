//! Kernel de alto nivel (HLE) inspirado en Horizon, el sistema de la Switch.
//!
//! Implementa un proceso con varios hilos (planificador cooperativo por prioridades sobre una sola CPU
//! emulada), tabla de handles, memoria del proceso, sincronización (mutex, variables de condición, eventos,
//! esperas por dirección) y comunicación por IPC con servicios HLE.
//!
//! Los números de SVC y las estructuras siguen la documentación pública de la Switch original; no hay
//! documentación pública del sistema de la Switch 2, de modo que se parte del ABI conocido.

pub mod ipc;
pub mod services;

use crate::cpu::{Cpu, CpuError, Exit};
use crate::memory::{Memory, PAGE_SIZE, PERM_R, PERM_RW};
use crate::formats::{nso, pfs};
use crate::nro;
use ipc::{parse_request, write_response, IpcResponse, TYPE_CLOSE, TYPE_CONTROL, TYPE_CONTROL_WITH_CONTEXT};
use services::{create, Service};
use std::collections::HashMap;

pub const TICKS_PER_SEC: u64 = 19_200_000;
/// Instrucciones de la CPU emulada que equivalen a un tick del contador de sistema.
const INSTR_PER_TICK: u64 = 50;
const SLICE: u64 = 20_000;

pub const HEAP_BASE: u64 = 0x2000_0000;
pub const HEAP_MAX: u64 = 0x1000_0000;
pub const ALIAS_BASE: u64 = 0x4000_0000;
pub const ALIAS_SIZE: u64 = 0x1000_0000;
pub const TLS_BASE: u64 = 0x0FD0_0000;
pub const TLS_SLOT: u64 = 0x200;
const HANDLE_BASE: u32 = 0xD000;
pub const CUR_THREAD_PSEUDO: u32 = 0xFFFF_8000;
pub const CUR_PROCESS_PSEUDO: u32 = 0xFFFF_8001;
const HAS_LISTENERS: u32 = 0x4000_0000;

/// Códigos de resultado del kernel (módulo 1), según la documentación pública.
pub mod res {
    pub const SUCCESS: u32 = 0;
    pub const OUT_OF_MEMORY: u32 = 0xD001;
    pub const INVALID_SIZE: u32 = 0xCA01;
    pub const INVALID_ADDRESS: u32 = 0xCC01;
    pub const OUT_OF_RESOURCE: u32 = 0xCE01;
    pub const INVALID_MEMORY_STATE: u32 = 0xD401;
    pub const INVALID_MEMORY_PERMISSIONS: u32 = 0xD801;
    pub const INVALID_COMBINATION: u32 = 0xE001;
    pub const INVALID_HANDLE: u32 = 0xE401;
    pub const TIMED_OUT: u32 = 0xEA01;
    pub const CANCELLED: u32 = 0xEC01;
    pub const NOT_FOUND: u32 = 0xF201;
    pub const NOT_IMPLEMENTED: u32 = 0xF001;
    pub const INVALID_STATE: u32 = 0xFA01;
}

enum Object {
    Thread(usize),
    Event(usize),
    Session(Box<dyn Service>),
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Runnable,
    Sleeping,
    WaitSync,
    WaitMutex { addr: u64 },
    WaitCondVar { key: u64, mutex: u64, tag: u32 },
    WaitAddress { addr: u64 },
    Dead,
}

struct Thread {
    id: u64,
    handle: u32,
    cpu: Cpu,
    state: State,
    priority: u32,
    tls: u64,
    wake_at: Option<u64>,
    wait_handles: Vec<u32>,
    wait_seq: u64,
    last_run: u64,
    /// Resultado que recibirá al reanudarse tras recuperar un mutex (p. ej. `TIMED_OUT` de una variable de condición).
    pending_result: u32,
}

enum Flow {
    Continue,
    Block,
    Yield,
}

/// Región del espacio de direcciones con su tipo de memoria (para `QueryMemory`).
#[derive(Clone, Copy)]
struct MemType {
    start: u64,
    end: u64,
    ty: u32,
}

pub struct Kernel {
    pub mem: Memory,
    handles: HashMap<u32, Object>,
    next_handle: u32,
    threads: Vec<Thread>,
    current: usize,
    events: Vec<bool>,
    heap_size: u64,
    clock: u64,
    clock_frac: u64,
    instructions: u64,
    pub debug_output: String,
    /// Eventos sin implementar o sospechosos (SVC/IPC desconocidos…), útiles al portar juegos.
    pub log: Vec<String>,
    exit_code: Option<u64>,
    seq: u64,
    next_thread_id: u64,
    aliases: Vec<(u64, u64, u64)>,
    mem_types: Vec<MemType>,
}

fn ns_to_ticks(ns: u64) -> u64 {
    (ns as u128 * TICKS_PER_SEC as u128 / 1_000_000_000) as u64
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            mem: Memory::new(),
            handles: HashMap::new(),
            next_handle: HANDLE_BASE,
            threads: Vec::new(),
            current: 0,
            events: Vec::new(),
            heap_size: 0,
            clock: 0,
            clock_frac: 0,
            instructions: 0,
            debug_output: String::new(),
            log: Vec::new(),
            exit_code: None,
            seq: 0,
            next_thread_id: 1,
            aliases: Vec::new(),
            mem_types: Vec::new(),
        }
    }

    fn new_handle(&mut self, o: Object) -> u32 {
        let h = self.next_handle;
        self.next_handle += 1;
        self.handles.insert(h, o);
        h
    }

    fn alloc_tls(&mut self) -> u64 {
        let tls = TLS_BASE + self.next_thread_id * TLS_SLOT;
        self.mem.map_perm(tls & !(PAGE_SIZE - 1), PAGE_SIZE, PERM_RW);
        tls
    }

    fn spawn(&mut self, pc: u64, arg: u64, sp: u64, priority: u32, runnable: bool) -> usize {
        let idx = self.threads.len();
        let id = self.next_thread_id;
        let tls = self.alloc_tls();
        self.next_thread_id += 1;
        let handle = self.new_handle(Object::Thread(idx));
        let mut cpu = Cpu::new();
        cpu.pc = pc;
        cpu.sp = sp;
        cpu.x[0] = arg;
        cpu.x[1] = handle as u64;
        cpu.tpidr_el0 = tls;
        self.threads.push(Thread {
            id,
            handle,
            cpu,
            state: if runnable { State::Runnable } else { State::Sleeping },
            priority,
            tls,
            wake_at: None,
            wait_handles: Vec::new(),
            wait_seq: 0,
            last_run: 0,
            pending_result: 0,
        });
        idx
    }

    /// Carga un `.nro` y crea el hilo principal.
    pub fn boot_nro(&mut self, data: &[u8]) -> Result<(), String> {
        let loaded = nro::load(data, &mut self.mem).map_err(|e| format!("NRO inválido: {e:?}"))?;
        self.mem_types.push(MemType { start: nro::NRO_BASE, end: nro::NRO_BASE + loaded.image_size, ty: 3 });
        self.mem_types.push(MemType { start: nro::STACK_TOP - nro::STACK_SIZE, end: nro::STACK_TOP, ty: 2 });
        let t = self.spawn(loaded.entry, 0, loaded.stack_top, 44, true);
        self.threads[t].cpu.x[1] = u64::MAX; // convenio de libnx: sin hilo principal «real»
        Ok(())
    }

    /// Arranca desde un ExeFS sin cifrar (PFS0 con `rtld`, `main`, `subsdk*` y `sdk`): carga los módulos NSO en orden,
    /// uno a continuación del otro, y empieza a ejecutar en el primero.
    pub fn boot_exefs(&mut self, exefs: &[u8]) -> Result<(), String> {
        let c = pfs::parse(exefs).map_err(|e| format!("ExeFS inválido: {e:?}"))?;
        let mut names: Vec<&str> = c.files.iter().map(|f| f.name.as_str()).filter(|n| *n == "rtld" || *n == "main" || *n == "sdk" || n.starts_with("subsdk")).collect();
        let rank = |n: &str| match n {
            "rtld" => (0, String::new()),
            "main" => (1, String::new()),
            "sdk" => (3, String::new()),
            other => (2, other.to_string()),
        };
        names.sort_by_key(|n| rank(n));
        if !names.contains(&"main") {
            return Err("el ExeFS no contiene «main»".into());
        }
        let mut base = nro::NRO_BASE;
        let first = base;
        for n in names {
            let module = nso::parse(c.get(exefs, n).unwrap()).map_err(|e| format!("{n}: {e:?}"))?;
            let end = module.load(&mut self.mem, base).map_err(|e| format!("{n}: {e:?}"))?;
            self.mem_types.push(MemType { start: base, end, ty: 3 });
            base = end;
        }
        self.mem.map_perm(nro::STACK_TOP - nro::STACK_SIZE, nro::STACK_SIZE, PERM_RW);
        self.mem_types.push(MemType { start: nro::STACK_TOP - nro::STACK_SIZE, end: nro::STACK_TOP, ty: 2 });
        let t = self.spawn(first, 0, nro::STACK_TOP, 44, true);
        self.threads[t].cpu.x[1] = u64::MAX;
        Ok(())
    }

    /// Carga un binario crudo (mapeado RWX en `base`) y crea el hilo principal; pensado para pruebas.
    pub fn boot_raw(&mut self, code: &[u8], base: u64) {
        self.mem.map(base, code.len() as u64 + 0x10_0000);
        self.mem.write(base, code).unwrap();
        self.mem.map_perm(nro::STACK_TOP - nro::STACK_SIZE, nro::STACK_SIZE, PERM_RW);
        self.spawn(base, 0, nro::STACK_TOP, 44, true);
    }

    pub fn now(&self) -> u64 {
        self.clock
    }

    pub fn instructions(&self) -> u64 {
        self.instructions
    }

    /// Ejecuta el proceso hasta `ExitProcess` (o hasta que todos los hilos terminen). Devuelve el código de salida.
    pub fn run(&mut self, max_instructions: u64) -> Result<u64, String> {
        loop {
            if let Some(code) = self.exit_code {
                return Ok(code);
            }
            if self.instructions >= max_instructions {
                return Err(format!("límite de {max_instructions} instrucciones superado"));
            }
            self.wake_timeouts();
            let Some(t) = self.pick_thread() else {
                if self.threads.iter().all(|t| t.state == State::Dead) {
                    return Ok(0);
                }
                if let Some(next) = self.threads.iter().filter_map(|t| t.wake_at).min() {
                    self.clock = self.clock.max(next);
                    continue;
                }
                let blocked: Vec<String> = self.threads.iter().filter(|t| t.state != State::Dead).map(|t| format!("hilo {} en {:?}", t.id, t.state)).collect();
                return Err(format!("interbloqueo: {}", blocked.join(", ")));
            };
            self.current = t;
            self.seq += 1;
            self.threads[t].last_run = self.seq;
            let mut cpu = std::mem::take(&mut self.threads[t].cpu);
            let mut n = 0u64;
            let mut fault: Option<String> = None;
            loop {
                if n >= SLICE {
                    break;
                }
                match cpu.step(&mut self.mem) {
                    Ok(None) => n += 1,
                    Ok(Some(Exit::Svc(num))) => {
                        n += 1;
                        match self.svc(t, &mut cpu, num) {
                            Ok(Flow::Continue) => {}
                            Ok(Flow::Block) | Ok(Flow::Yield) => break,
                            Err(e) => {
                                fault = Some(e);
                                break;
                            }
                        }
                        if self.exit_code.is_some() {
                            break;
                        }
                    }
                    Err(e) => {
                        fault = Some(match e {
                            CpuError::Unimplemented { pc, insn } => format!("instrucción no implementada {insn:#010x} en pc={pc:#x}"),
                            CpuError::Mem(m) => format!("fallo de memoria {m:?} en pc={:#x}", cpu.pc),
                        });
                        break;
                    }
                }
            }
            self.threads[t].cpu = cpu;
            self.instructions += n;
            self.clock_frac += n;
            self.clock += self.clock_frac / INSTR_PER_TICK;
            self.clock_frac %= INSTR_PER_TICK;
            if let Some(f) = fault {
                return Err(format!("hilo {}: {f}", self.threads[t].id));
            }
        }
    }

    /// Hilo ejecutable de mayor prioridad (número menor); entre iguales, el que lleva más tiempo sin correr.
    fn pick_thread(&self) -> Option<usize> {
        self.threads
            .iter()
            .enumerate()
            .filter(|(_, t)| t.state == State::Runnable)
            .min_by_key(|(_, t)| (t.priority, t.last_run))
            .map(|(i, _)| i)
    }

    fn wake(&mut self, t: usize, x0: u32, x1: Option<u64>) {
        let th = &mut self.threads[t];
        th.state = State::Runnable;
        th.wake_at = None;
        th.cpu.x[0] = x0 as u64;
        if let Some(v) = x1 {
            th.cpu.x[1] = v;
        }
    }

    fn wake_timeouts(&mut self) {
        let now = self.clock;
        for t in 0..self.threads.len() {
            let Some(at) = self.threads[t].wake_at else { continue };
            if at > now {
                continue;
            }
            match self.threads[t].state.clone() {
                State::Sleeping => self.wake(t, res::SUCCESS, None),
                State::WaitSync | State::WaitAddress { .. } => self.wake(t, res::TIMED_OUT, None),
                State::WaitCondVar { .. } => self.cv_resume(t, res::TIMED_OUT),
                _ => self.threads[t].wake_at = None,
            }
        }
    }

    fn is_signaled(&self, h: u32) -> Option<bool> {
        match self.handles.get(&h)? {
            Object::Thread(i) => Some(self.threads[*i].state == State::Dead),
            Object::Event(e) => Some(self.events[*e]),
            Object::Session(_) => Some(false),
        }
    }

    /// Reevalúa a los hilos que esperan por objetos (tras señalizar un evento o terminar un hilo).
    fn check_waiters(&mut self) {
        for t in 0..self.threads.len() {
            if self.threads[t].state != State::WaitSync {
                continue;
            }
            let hs = self.threads[t].wait_handles.clone();
            if let Some(idx) = hs.iter().position(|h| self.is_signaled(*h) == Some(true)) {
                self.wake(t, res::SUCCESS, Some(idx as u64));
            }
        }
    }

    // ---------------------------------------------------------------- mutex y variables de condición
    fn read32(&self, addr: u64) -> Option<u32> {
        self.mem.read_uint(addr, 4).ok().map(|v| v as u32)
    }

    fn waiters_of(&self, f: impl Fn(&State) -> bool) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.threads.len()).filter(|i| f(&self.threads[*i].state)).collect();
        v.sort_by_key(|i| (self.threads[*i].priority, self.threads[*i].wait_seq));
        v
    }

    /// Libera el mutex: cede la propiedad al primer hilo en espera o lo deja a 0.
    fn mutex_unlock(&mut self, addr: u64) -> Result<(), u32> {
        let waiters = self.waiters_of(|s| matches!(s, State::WaitMutex { addr: a } if *a == addr));
        let new = match waiters.first() {
            Some(&w) => self.threads[w].handle | if waiters.len() > 1 { HAS_LISTENERS } else { 0 },
            None => 0,
        };
        self.mem.write_uint(addr, 4, new as u64).map_err(|_| res::INVALID_ADDRESS)?;
        if let Some(&w) = waiters.first() {
            let r = std::mem::take(&mut self.threads[w].pending_result);
            self.wake(w, r, None);
        }
        Ok(())
    }

    /// Un hilo de una variable de condición se reanuda: intenta recuperar el mutex.
    fn cv_resume(&mut self, t: usize, result: u32) {
        let State::WaitCondVar { mutex, tag, .. } = self.threads[t].state.clone() else { return };
        let val = self.read32(mutex).unwrap_or(0);
        if val == 0 {
            let _ = self.mem.write_uint(mutex, 4, tag as u64);
            self.wake(t, result, None);
        } else {
            let _ = self.mem.write_uint(mutex, 4, (val | HAS_LISTENERS) as u64);
            self.seq += 1;
            let th = &mut self.threads[t];
            th.state = State::WaitMutex { addr: mutex };
            th.pending_result = result;
            th.wake_at = None;
            th.wait_seq = self.seq;
        }
    }

    // -------------------------------------------------------------------------------- SVC
    fn thread_of(&self, h: u32) -> Option<usize> {
        if h == CUR_THREAD_PSEUDO {
            return Some(self.current);
        }
        match self.handles.get(&h)? {
            Object::Thread(i) => Some(*i),
            _ => None,
        }
    }

    fn block(&mut self, t: usize, state: State, timeout_ns: i64) {
        self.seq += 1;
        let now = self.clock;
        let th = &mut self.threads[t];
        th.state = state;
        th.wait_seq = self.seq;
        th.wake_at = if timeout_ns >= 0 { Some(now + ns_to_ticks(timeout_ns as u64).max(1)) } else { None };
    }

    fn svc(&mut self, t: usize, cpu: &mut Cpu, num: u32) -> Result<Flow, String> {
        let a = cpu.x;
        let set = |cpu: &mut Cpu, code: u32| cpu.x[0] = code as u64;
        match num {
            // SetHeapSize(size) -> (result, base)
            0x01 => {
                let size = a[1];
                if size % 0x20_0000 != 0 {
                    set(cpu, res::INVALID_SIZE);
                } else if size > HEAP_MAX {
                    set(cpu, res::OUT_OF_MEMORY);
                } else {
                    if size > self.heap_size {
                        self.mem.map_perm(HEAP_BASE + self.heap_size, size - self.heap_size, PERM_RW);
                    } else if size < self.heap_size {
                        self.mem.unmap(HEAP_BASE + size, self.heap_size - size);
                    }
                    self.heap_size = size;
                    self.mem_types.retain(|m| m.start != HEAP_BASE);
                    self.mem_types.push(MemType { start: HEAP_BASE, end: HEAP_BASE + size, ty: 5 });
                    set(cpu, res::SUCCESS);
                    cpu.x[1] = HEAP_BASE;
                }
            }
            // SetMemoryPermission(addr, size, perm)
            0x02 => {
                let (addr, size, perm) = (a[0], a[1], a[2] as u8);
                let r = if addr % PAGE_SIZE != 0 {
                    res::INVALID_ADDRESS
                } else if size == 0 || size % PAGE_SIZE != 0 {
                    res::INVALID_SIZE
                } else if !matches!(perm, 0 | 1 | 3 | 5) {
                    res::INVALID_MEMORY_PERMISSIONS
                } else if self.mem.protect(addr, size, perm).is_err() {
                    res::INVALID_MEMORY_STATE
                } else {
                    res::SUCCESS
                };
                set(cpu, r);
            }
            // SetMemoryAttribute: sin efecto
            0x03 => set(cpu, res::SUCCESS),
            // MapMemory(dst, src, size) / UnmapMemory(dst, src, size)
            0x04 => {
                let (dst, src, size) = (a[0], a[1], a[2]);
                let ok_align = dst % PAGE_SIZE == 0 && src % PAGE_SIZE == 0;
                let r = if !ok_align {
                    res::INVALID_ADDRESS
                } else if size == 0 || size % PAGE_SIZE != 0 {
                    res::INVALID_SIZE
                } else if dst < ALIAS_BASE || dst + size > ALIAS_BASE + ALIAS_SIZE || self.mem.perm_at(dst).is_some() {
                    res::INVALID_MEMORY_STATE
                } else {
                    let mut buf = vec![0u8; size as usize];
                    if self.mem.query(src).mapped && self.mem.query(src).size >= size && self.mem.perm_at(src).unwrap_or(0) & PERM_R != 0 && self.mem.read(src, &mut buf).is_ok() {
                        self.mem.map_perm(dst, size, PERM_RW);
                        self.mem.write(dst, &buf).unwrap();
                        self.mem.protect(src, size, 0).ok();
                        self.aliases.push((dst, src, size));
                        res::SUCCESS
                    } else {
                        res::INVALID_MEMORY_STATE
                    }
                };
                set(cpu, r);
            }
            0x05 => {
                let (dst, src, size) = (a[0], a[1], a[2]);
                let r = match self.aliases.iter().position(|x| *x == (dst, src, size)) {
                    Some(i) => {
                        let mut buf = vec![0u8; size as usize];
                        self.mem.read(dst, &mut buf).ok();
                        self.mem.protect(src, size, PERM_RW).ok();
                        self.mem.write(src, &buf).ok();
                        self.mem.unmap(dst, size);
                        self.aliases.remove(i);
                        res::SUCCESS
                    }
                    None => res::INVALID_MEMORY_STATE,
                };
                set(cpu, r);
            }
            // QueryMemory(meminfo*, pageinfo*, addr) -> (result, pageinfo)
            0x06 => {
                let (out, addr) = (a[0], a[2]);
                let reg = self.mem.query(addr);
                let ty = if !reg.mapped {
                    0
                } else {
                    self.mem_types.iter().find(|m| reg.start >= m.start && reg.start < m.end).map_or(
                        if (TLS_BASE..TLS_BASE + 0x10_0000).contains(&reg.start) { 12 } else if reg.start >= ALIAS_BASE && reg.start < ALIAS_BASE + ALIAS_SIZE { 11 } else { 2 },
                        |m| if m.ty == 3 && reg.perm & 2 != 0 { 4 } else { m.ty },
                    )
                };
                let mut info = Vec::with_capacity(0x28);
                info.extend_from_slice(&reg.start.to_le_bytes());
                info.extend_from_slice(&reg.size.to_le_bytes());
                info.extend_from_slice(&ty.to_le_bytes());
                info.extend_from_slice(&0u32.to_le_bytes());
                info.extend_from_slice(&(reg.perm as u32).to_le_bytes());
                info.extend_from_slice(&[0u8; 12]);
                let r = if self.mem.write(out, &info).is_ok() { res::SUCCESS } else { res::INVALID_ADDRESS };
                set(cpu, r);
                cpu.x[1] = 0;
            }
            // ExitProcess (el código de salida se toma de x0 para facilitar las pruebas)
            0x07 => {
                self.exit_code = Some(a[0]);
                return Ok(Flow::Block);
            }
            // CreateThread(entry, arg, stack_top, priority, core) -> (result, handle)
            0x08 => {
                let prio = a[3] as u32;
                if prio > 63 {
                    set(cpu, res::INVALID_COMBINATION);
                } else if self.threads.len() >= 0x200 {
                    set(cpu, res::OUT_OF_RESOURCE);
                } else {
                    let idx = self.spawn(a[0], a[1], a[2], prio, false);
                    set(cpu, res::SUCCESS);
                    cpu.x[1] = self.threads[idx].handle as u64;
                }
            }
            // StartThread(handle)
            0x09 => match self.thread_of(a[0] as u32) {
                Some(i) if self.threads[i].state == State::Sleeping && self.threads[i].wake_at.is_none() && i != t => {
                    self.threads[i].state = State::Runnable;
                    set(cpu, res::SUCCESS);
                }
                Some(_) => set(cpu, res::INVALID_STATE),
                None => set(cpu, res::INVALID_HANDLE),
            },
            // ExitThread
            0x0A => {
                self.threads[t].state = State::Dead;
                self.check_waiters();
                if self.threads.iter().all(|x| x.state == State::Dead) {
                    self.exit_code = Some(0);
                }
                return Ok(Flow::Block);
            }
            // SleepThread(ns)
            0x0B => {
                let ns = a[0] as i64;
                if ns <= 0 {
                    return Ok(Flow::Yield);
                }
                self.block(t, State::Sleeping, ns);
                return Ok(Flow::Block);
            }
            // GetThreadPriority / SetThreadPriority
            0x0C => match self.thread_of(a[1] as u32) {
                Some(i) => {
                    set(cpu, res::SUCCESS);
                    cpu.x[1] = self.threads[i].priority as u64;
                }
                None => set(cpu, res::INVALID_HANDLE),
            },
            0x0D => match self.thread_of(a[0] as u32) {
                Some(i) if a[1] <= 63 => {
                    self.threads[i].priority = a[1] as u32;
                    set(cpu, res::SUCCESS);
                    return Ok(Flow::Yield);
                }
                Some(_) => set(cpu, res::INVALID_COMBINATION),
                None => set(cpu, res::INVALID_HANDLE),
            },
            // GetThreadCoreMask / SetThreadCoreMask / GetCurrentProcessorNumber
            0x0E => {
                set(cpu, res::SUCCESS);
                cpu.x[1] = 0;
                cpu.x[2] = 1;
            }
            0x0F => set(cpu, res::SUCCESS),
            0x10 => cpu.x[0] = 0,
            // SignalEvent / ClearEvent / ResetSignal
            0x11 | 0x12 | 0x17 => match self.handles.get(&(a[0] as u32)) {
                Some(Object::Event(e)) => {
                    let e = *e;
                    match num {
                        0x11 => {
                            self.events[e] = true;
                            self.check_waiters();
                            set(cpu, res::SUCCESS);
                        }
                        0x12 => {
                            self.events[e] = false;
                            set(cpu, res::SUCCESS);
                        }
                        _ => {
                            let was = self.events[e];
                            self.events[e] = false;
                            set(cpu, if was { res::SUCCESS } else { res::INVALID_STATE });
                        }
                    }
                }
                _ => set(cpu, res::INVALID_HANDLE),
            },
            // CloseHandle
            0x16 => {
                let r = if self.handles.remove(&(a[0] as u32)).is_some() { res::SUCCESS } else { res::INVALID_HANDLE };
                set(cpu, r);
            }
            // WaitSynchronization(handles*, count, timeout) -> (result, index)
            0x18 => {
                let (ptr, n, timeout) = (a[1], a[2] as usize, a[3] as i64);
                if n > 0x40 {
                    set(cpu, res::OUT_OF_RESOURCE);
                } else {
                    let mut hs = Vec::with_capacity(n);
                    for i in 0..n {
                        match self.mem.read_uint(ptr + 4 * i as u64, 4) {
                            Ok(h) => hs.push(h as u32),
                            Err(_) => {
                                set(cpu, res::INVALID_ADDRESS);
                                return Ok(Flow::Continue);
                            }
                        }
                    }
                    if hs.iter().any(|h| self.is_signaled(*h).is_none()) {
                        set(cpu, res::INVALID_HANDLE);
                    } else if let Some(idx) = hs.iter().position(|h| self.is_signaled(*h) == Some(true)) {
                        set(cpu, res::SUCCESS);
                        cpu.x[1] = idx as u64;
                    } else if timeout == 0 {
                        set(cpu, res::TIMED_OUT);
                    } else {
                        self.threads[t].wait_handles = hs;
                        self.block(t, State::WaitSync, timeout);
                        return Ok(Flow::Block);
                    }
                }
            }
            // CancelSynchronization(thread handle)
            0x19 => match self.thread_of(a[0] as u32) {
                Some(i) => {
                    if self.threads[i].state == State::WaitSync {
                        self.wake(i, res::CANCELLED, Some(0));
                    }
                    set(cpu, res::SUCCESS);
                }
                None => set(cpu, res::INVALID_HANDLE),
            },
            // ArbitrateLock(owner handle, mutex addr, requester handle)
            0x1A => {
                let (owner, addr) = (a[0] as u32, a[1]);
                if addr % 4 != 0 {
                    set(cpu, res::INVALID_ADDRESS);
                } else {
                    match self.read32(addr) {
                        None => set(cpu, res::INVALID_ADDRESS),
                        Some(v) if v != (owner | HAS_LISTENERS) => set(cpu, res::SUCCESS),
                        Some(_) => {
                            self.threads[t].pending_result = 0;
                            self.block(t, State::WaitMutex { addr }, -1);
                            return Ok(Flow::Block);
                        }
                    }
                }
            }
            // ArbitrateUnlock(mutex addr)
            0x1B => {
                if a[0] % 4 != 0 {
                    set(cpu, res::INVALID_ADDRESS);
                } else {
                    let r = self.mutex_unlock(a[0]).err().unwrap_or(res::SUCCESS);
                    set(cpu, r);
                }
            }
            // WaitProcessWideKeyAtomic(mutex addr, cv key, tag, timeout)
            0x1C => {
                let (addr, key, tag, timeout) = (a[0], a[1], a[2] as u32, a[3] as i64);
                if addr % 4 != 0 {
                    set(cpu, res::INVALID_ADDRESS);
                } else if let Err(e) = self.mutex_unlock(addr) {
                    set(cpu, e);
                } else if timeout == 0 {
                    // sin espera: recupera el mutex y devuelve TimedOut
                    self.block(t, State::WaitCondVar { key, mutex: addr, tag }, -1);
                    self.cv_resume(t, res::TIMED_OUT);
                    if self.threads[t].state == State::Runnable {
                        let r = self.threads[t].cpu.x[0];
                        set(cpu, r as u32);
                        return Ok(Flow::Continue);
                    }
                    return Ok(Flow::Block);
                } else {
                    self.block(t, State::WaitCondVar { key, mutex: addr, tag }, timeout);
                    return Ok(Flow::Block);
                }
            }
            // SignalProcessWideKey(key, count)
            0x1D => {
                let (key, count) = (a[0], a[1] as u32 as i32);
                let waiters = self.waiters_of(|s| matches!(s, State::WaitCondVar { key: k, .. } if *k == key));
                let take = if count <= 0 { waiters.len() } else { (count as usize).min(waiters.len()) };
                for &w in &waiters[..take] {
                    self.cv_resume(w, res::SUCCESS);
                }
                set(cpu, res::SUCCESS);
            }
            // GetSystemTick
            0x1E => cpu.x[0] = self.clock,
            // ConnectToNamedPort(name*) -> (result, handle)
            0x1F => {
                let mut name = [0u8; 12];
                if self.mem.read(a[1], &mut name).is_err() {
                    set(cpu, res::INVALID_ADDRESS);
                } else {
                    let name: String = name.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
                    match create(&name) {
                        Some(s) => {
                            let h = self.new_handle(Object::Session(s));
                            set(cpu, res::SUCCESS);
                            cpu.x[1] = h as u64;
                        }
                        None => {
                            self.log.push(format!("ConnectToNamedPort: puerto desconocido «{name}»"));
                            set(cpu, res::NOT_FOUND);
                        }
                    }
                }
            }
            // SendSyncRequest(session handle)
            0x21 => {
                let r = self.send_sync_request(t, a[0] as u32);
                set(cpu, r);
            }
            // GetProcessId / GetThreadId
            0x24 => {
                set(cpu, res::SUCCESS);
                cpu.x[1] = 0x51;
            }
            0x25 => match self.thread_of(a[1] as u32) {
                Some(i) => {
                    set(cpu, res::SUCCESS);
                    cpu.x[1] = self.threads[i].id;
                }
                None => set(cpu, res::INVALID_HANDLE),
            },
            // Break(reason, info, size): el bit 31 indica una notificación no fatal
            0x26 => {
                if a[0] & 0x8000_0000 != 0 {
                    self.log.push(format!("svcBreak no fatal (reason={:#x})", a[0]));
                } else {
                    return Err(format!("svcBreak (reason={:#x}, info={:#x}, size={:#x})", a[0], a[1], a[2]));
                }
            }
            // OutputDebugString(ptr, size)
            0x27 => {
                let mut buf = vec![0u8; a[1] as usize];
                if self.mem.read(a[0], &mut buf).is_err() {
                    set(cpu, res::INVALID_ADDRESS);
                } else {
                    self.debug_output.push_str(&String::from_utf8_lossy(&buf));
                    set(cpu, res::SUCCESS);
                }
            }
            // GetInfo(info_type, handle, sub_id) -> (result, value)
            0x29 => {
                let (ty, _handle, _id) = (a[1], a[2], a[3]);
                let v = match ty {
                    0 => 0xF,
                    1 => u64::MAX,
                    2 => ALIAS_BASE,
                    3 => ALIAS_SIZE,
                    4 => HEAP_BASE,
                    5 => HEAP_MAX,
                    6 | 20 => 0xC000_0000,
                    7 | 21 => self.heap_size + 0x10_0000,
                    12 => 0x0800_0000,
                    13 => 0x7FF_8000_0000,
                    14 => ALIAS_BASE,
                    15 => ALIAS_SIZE,
                    18 => 0,
                    19 => 0,
                    22 => 1,
                    _ => {
                        self.log.push(format!("GetInfo: tipo {ty} no soportado"));
                        set(cpu, res::INVALID_COMBINATION);
                        return Ok(Flow::Continue);
                    }
                };
                set(cpu, res::SUCCESS);
                cpu.x[1] = v;
            }
            // WaitForAddress(addr, arb_type, value, timeout)
            0x34 => {
                let (addr, ty, value, timeout) = (a[0], a[1] as u32, a[2] as u32 as i32, a[3] as i64);
                let Some(cur) = self.read32(addr).map(|v| v as i32) else {
                    set(cpu, res::INVALID_ADDRESS);
                    return Ok(Flow::Continue);
                };
                let wait = match ty {
                    0 => cur < value,
                    1 => {
                        let w = cur < value;
                        if w {
                            let _ = self.mem.write_uint(addr, 4, (cur - 1) as u32 as u64);
                        }
                        w
                    }
                    2 => cur == value,
                    _ => {
                        set(cpu, res::INVALID_ADDRESS);
                        return Ok(Flow::Continue);
                    }
                };
                if !wait {
                    set(cpu, res::INVALID_STATE);
                } else if timeout == 0 {
                    set(cpu, res::TIMED_OUT);
                } else {
                    self.block(t, State::WaitAddress { addr }, timeout);
                    return Ok(Flow::Block);
                }
            }
            // SignalToAddress(addr, signal_type, value, count)
            0x35 => {
                let (addr, ty, value, count) = (a[0], a[1] as u32, a[2] as u32 as i32, a[3] as u32 as i32);
                let cur = self.read32(addr).map(|v| v as i32);
                if cur.is_none() {
                    set(cpu, res::INVALID_ADDRESS);
                    return Ok(Flow::Continue);
                }
                if ty == 1 {
                    if cur != Some(value) {
                        set(cpu, res::INVALID_STATE);
                        return Ok(Flow::Continue);
                    }
                    let _ = self.mem.write_uint(addr, 4, (value + 1) as u32 as u64);
                }
                let waiters = self.waiters_of(|s| matches!(s, State::WaitAddress { addr: x } if *x == addr));
                let take = if count <= 0 { waiters.len() } else { (count as usize).min(waiters.len()) };
                for &w in &waiters[..take] {
                    self.wake(w, res::SUCCESS, None);
                }
                set(cpu, res::SUCCESS);
            }
            // CreateEvent() -> (result, writable handle, readable handle)
            0x45 => {
                self.events.push(false);
                let id = self.events.len() - 1;
                let (w, r) = (self.new_handle(Object::Event(id)), self.new_handle(Object::Event(id)));
                set(cpu, res::SUCCESS);
                cpu.x[1] = w as u64;
                cpu.x[2] = r as u64;
            }
            n => {
                self.log.push(format!("SVC {n:#x} no implementada (pc={:#x})", cpu.pc.wrapping_sub(4)));
                set(cpu, res::NOT_IMPLEMENTED);
            }
        }
        Ok(Flow::Continue)
    }

    // ---------------------------------------------------------------------------------- IPC
    fn send_sync_request(&mut self, t: usize, handle: u32) -> u32 {
        let tls = self.threads[t].tls;
        let req = match parse_request(&self.mem, tls) {
            Ok(r) => r,
            Err(_) => return res::INVALID_ADDRESS,
        };
        let Some(Object::Session(mut svc)) = self.handles.remove(&handle) else {
            return res::INVALID_HANDLE;
        };
        if req.kind == TYPE_CLOSE {
            return res::SUCCESS; // la sesión ya se retiró de la tabla
        }
        let resp = if matches!(req.kind, TYPE_CONTROL | TYPE_CONTROL_WITH_CONTEXT) {
            // comandos de control: CloneCurrentObject (2), QueryPointerBufferSize (3), CloneCurrentObjectEx (4)
            match req.command {
                2 | 4 => match create(svc.name()) {
                    Some(c) => {
                        let mut r = IpcResponse::ok();
                        r.new_sessions.push(c);
                        r
                    }
                    None => IpcResponse::err(services::RESULT_UNKNOWN_COMMAND),
                },
                3 => {
                    let mut r = IpcResponse::ok();
                    r.push_u32(0x8000);
                    r
                }
                c => {
                    self.log.push(format!("{}: comando de control {c} no soportado", svc.name()));
                    IpcResponse::err(services::RESULT_UNKNOWN_COMMAND)
                }
            }
        } else {
            let r = svc.call(&req);
            if r.result == services::RESULT_UNKNOWN_COMMAND {
                self.log.push(format!("{}: comando {} no implementado", svc.name(), req.command));
            }
            r
        };
        self.handles.insert(handle, Object::Session(svc));
        let mut resp = resp;
        let mut moved = std::mem::take(&mut resp.move_handles);
        for s in std::mem::take(&mut resp.new_sessions) {
            moved.push(self.new_handle(Object::Session(s)));
        }
        let _ = write_response(&mut self.mem, tls, &resp, &moved);
        res::SUCCESS
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}
