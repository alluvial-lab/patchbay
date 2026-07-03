---
id: idea-agent-to-agent-mesh-seam
created: 2026-07-02
updated: 2026-07-03
tags: [adapter, coordination, protocol]
---

# Agent-to-agent messaging seam (local mesh)

remote_pi ships a local agent mesh: multiple Pi agents on one machine discover
each other via a Unix Domain Socket broker (leader-elected), then message each
other with `agent_send` (fire-and-forget) / `agent_request` (req/resp). It is
useful — it is the substrate for parallel multi-agent workflows on one box
(microsecond latency, no network config), and the harness this project runs in
already exposes the mesh tools (`agent_send`, `list_peers`, the `agent-network`
skill).

## Tension with patchbay's core posture

remote_pi's mesh is deliberately *around* the relay — direct UDS, no
persistence, no authority check. patchbay's core is deliberately *the* path —
durable acceptance, LSN assignment, authority check, lifecycle tracking. So
agent-to-agent traffic in patchbay has a genuine design fork, not a clean slot:

- **α — through the core:** every agent→agent message is a command with a
  grant, LSN, `CommandState`. Durable, authorized, auditable. Cost: latency
  and ceremony the local mesh exists to avoid.
- **β — around-core local mesh (remote_pi style):** direct UDS, bypassing
  durability. Fast, ergonomic. Cost: not durably recorded, not
  authority-checked, not recoverable — contradicts "an accepted command cannot
  vanish silently" *for that traffic class*.
- **γ — two tiers:** durable agent-to-agent *commands* (coordination, handoffs,
  crash-survival) through the core; ephemeral agent-to-agent *messages* (quick
  questions, status pings) via a local mesh seam. Cost: two messaging semantics;
  must classify per use.

## Why it's parked, not designed

The authority substrate is generic enough to host agent subjects (an agent is
already a first-class `actor`; grants already accept any actor as subject;
`CompoundIssuer` already models a non-human principal). So nothing forecloses
it. But resolving α/β/γ is real design work that depends on the
extension-seams classification discipline (`feature-extension-seams-non-foreclosure`)
landing first. Park until that feature clarifies which traffic classes are
durable vs ephemeral.

## Concrete reference

remote_pi broker/mesh implementation:
`pi-extension/src/session/broker.ts`, `mesh_node.ts`, `leader_election.ts`,
`broker_remote.ts`. PROTOCOL.md "Camadas do protocolo" / "Cross-PC routing"
sections describe the envelope + ACK model.
