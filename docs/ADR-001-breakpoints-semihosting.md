# ADR-001: Breakpoint policy and semihosting (design spike)

**Status:** Accepted (design only — no implementation commitment)  
**Date:** 2026-03-27  
**Context:** Roadmap issue “Phase 2 — Breakpoint policy and semihosting”

## Context

`rsgdb` sits between GDB and a debug backend (OpenOCD, probe-rs GDB server, pyOCD, etc.) and forwards RSP. Those backends already implement software/hardware breakpoints, stepping, and target-specific behavior. **Semihosting** (e.g. on ARM) involves halt reasons, syscall conventions, and sometimes monitor or semihosting-enable sequences — not all of which are visible as simple opaque RSP bytes.

## Goals

- Align with what **OpenOCD / probe-rs already do**; avoid duplicating or fighting backend policy.
- Decide where future logic could live: **RSP MITM** in `rsgdb` vs **backend configuration** outside the proxy.

## Breakpoint policy — options

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A. Transparent proxy** | Forward all RSP unchanged; `Z`/`z` pass through. | Matches current architecture; no semantic drift from backend. | No “smart” breakpoint naming in the proxy. |
| **B. MITM policy / annotation** | Log and optionally constrain `Z`/`z` (e.g. annotate with SVD symbol, enforce HW BP budget if backend exposes it). | Observability; optional guardrails. | Requires capability discovery or config; easy to get wrong vs real hardware. |
| **C. Full proxy-side breakpoints** | Implement breakpoint state in `rsgdb` and rewrite traffic. | Theoretical flexibility. | Duplicates backend; high risk; **not recommended** near term. |

**Recommendation:** Keep **A** as the default product behavior. Add **B**-style **logging/annotation** only (similar to SVD memory labels) unless a concrete use case requires enforcement. Any **enforcement** should be gated on explicit backend capability metadata, not guessed.

## Semihosting — options

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A. Pass-through** | Forward RSP; GDB + backend handle semihosting. | Simple; matches dumb proxy role. | Some setups need semihosting-aware GDB or backend flags. |
| **B. Decode in proxy** | Parse stop replies / monitor packets for syscall numbers; log or react. | Rich diagnostics; pairs with session recording. | ARM-specific branches; must not break opaque forwarding for other arches. |
| **C. Backend-only** | Document OpenOCD / probe-rs commands for semihosting; no RSP changes. | Clear separation. | Out of scope for `rsgdb` code paths. |

**Recommendation:** Default **A** + **C** (documentation). Pursue **B** only if we need **structured syscall trace** in recorded sessions and accept ARM-specific maintenance.

## Integration points (when implementing)

- **RSP MITM:** `ProxySession` — same layer as SVD hooks: parse `GdbCommand` for `Z`/`z` (optional logging), never rewrite without explicit feature flag.
- **Backend API:** There is no stable in-process API to probe-rs/OpenOCD from `rsgdb` today; orchestration is out of band (CLI, config files). Future “policy” should not assume direct backend callbacks without a defined integration.

## Risks

- **Policy mismatch:** Rewriting `Z`/`z` without matching the probe’s limits causes subtle failures.
- **Semihosting without architecture context:** Misleading logs or wrong conclusions from partial RSP.

## Out of scope for this ADR

- Implementing breakpoint rewriting or semihosting decode in Rust.
- Changing default pass-through behavior.

## References

- GDB Remote Serial Protocol (RSP), `Z`/`z` packets.
- ARM semihosting (e.g. ARM DUI, A-profile semihosting).
- CMSIS-SVD (related to annotation only; not breakpoints).
