# Modal One

An experimental WebAssembly kernel for RISC-V, written from scratch in Rust.

[日本語](README.ja.md)

### What is this?

Modal One is a minimal kernel that uses WebAssembly as its only execution environment. Instead of hardware-based process isolation, it relies on Wasm validation and sandboxing. Userland programs are Wasm modules; there is no other binary format.

This is a personal, long-term learning project. It is developed slowly and by hand. Expect it to take years, and expect it to be incomplete for most of that time.

### Status

**Pre-alpha. Nothing runs yet.** The repository currently contains design documents only.

Rough roadmap (non-binding):

| Phase | Goal | Status |
|-------|------|--------|
| 0 | `no_std` Wasm core interpreter (decoder / validator / executor), running on the host | not started |
| 1 | Boot on bare-metal RISC-V (QEMU `virt`): UART, allocator, interpreter | not started |
| 2 | Minimal hand-written WASI subset; run a `wasm32-wasip1` "Hello, World!" | not started |
| 3 | Inter-module calls (IPC) and measurement of sandbox boundary costs | not started |

### Design constraints

- **Interpreter only.** No JIT.
- **`no_std` from day one.** The execution path depends on `core` and `alloc` only.
- **Target:** riscv64, QEMU `virt` machine. Physical hardware is out of scope.
- **Wasm 1.0 (MVP) subset first.** Features from Wasm 2.0/3.0 are adopted individually and documented in ADRs.
- **Thin system-interface layer.** WASI support starts as a small hand-written p1-style subset, kept separate from the core so it can track the evolving WASI / Component Model standards.
- **Single-core for now.** SMP is not a current goal but is deliberately not ruled out.

### Hand-written, with defined exceptions

The execution path — binary decoder (including LEB128), validator, interpreter, kernel, WASI implementation — is written by hand, without implementation crates such as `wasmparser`.

Tooling is a different matter. The test harness uses the official WebAssembly spec test suite (and a wast parser to drive it), guest programs are built with the standard Rust `wasm32` targets, and [wasmtime](https://github.com/bytecodealliance/wasmtime) serves as a differential-testing oracle and as reference reading.

AI tools are used for study, design discussion, and review — not for generating the execution path.

### Non-goals

- Production use.
- JIT compilation.
- Support for physical hardware.
- Competing with existing runtimes or operating systems.

### Prior art and references

Modal One draws on prior work in this area, including [k23](https://github.com/JonasKruckenberg/k23), [wasmtime](https://github.com/bytecodealliance/wasmtime), [Theseus](https://github.com/theseus-os/Theseus), [Mewz](https://github.com/mewz-project/mewz), and the [WebAssembly specification](https://webassembly.github.io/spec/). These are references, not competitors; if you need something that works, use them.

### Documentation

Decisions are recorded as ADRs under `docs/adr/` (written in Japanese). ADR-000001 defines the project's principles.

### License

Dual-licensed under MIT or Apache-2.0, at your option.