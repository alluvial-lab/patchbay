---
id: idea-harvest-remote-pi-extension-as-adapter
created: 2026-07-02
updated: 2026-07-03
tags: [adapter, pi]
---

# Harvest remote_pi's pi-extension as Patchbay's Pi adapter

remote_pi's `pi-extension/` (Node + TypeScript) is a Pi SDK extension that
knows how to drive Pi: session discovery, lifecycle, transcript projection,
tool-call streaming. Patchbay's first adapter is Pi, so this is the obvious
source of Pi know-how.

## What harvests (Pi-facing session work)

- `src/session/*` — session gating, turn state, transcript event log,
  transcript projection, cwd lock, SDK session projection.
- `src/session/transcript_projection.ts` — Pi turn → app projection (the
  delivery/stream projection Patchbay's UX needs).
- The Pi lifecycle know-how encoded across the session layer.

## What does NOT harvest (replaces, not reuses)

- `src/transport/relay_client.ts`, `pairing/*`, `crypto.js` — these implement
  the *stateless relay + QR pairing + Ed25519 owner-key* model. Patchbay
  replaces that with a *stateful coordination core + TS web server +
  operator-session/CSRF/grant* model. Reusing them imports the wrong trust
  topology.
- `src/session/broker.ts`, `mesh_node.ts`, `leader_election.ts` — the local
  agent mesh; see `idea-agent-to-agent-mesh-seam` (deferred, separate seam).

## Caveat — re-housing

The pi-extension is a Pi SDK extension (`ExtensionFactory` default export,
`pi.on("tool_call")`, `@earendil-works/pi-coding-agent`). Patchbay's adapter
port is adapter-neutral (PROTOCOL §Adapter capabilities: "not making Pi the
core ontology"). So the Pi-specific session work harvests, but it must be
re-housed behind Patchbay's adapter port — harvesting the Pi-know-how, not the
extension shape. That is real adapter-implementation work, not a copy.

## When to pick up

When the Pi adapter is implemented (after the v0 protocol contracts
`feature-protocol-idl-and-conformance` lands, so the adapter has a generated
contract to speak). Reference the extension at
`/home/agent/projects/remote_pi/pi-extension/`.
