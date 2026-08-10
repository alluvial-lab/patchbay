---
source_handle: keyring-incident
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@cc0ba69aa65720466a5d985eb4f378c4634795d6:.work/session-notes/2026-08-03-patchbay-keyring-loss-mesh-revoke-incident.md
provenance: source-direct
---
# Source attestation: keyring-loss incident

Paraphrased summary: The 2026-08-03 incident occurred on the `patchbay` workstation while running outpost-pi. A `KeyRevoked` platform-keyring failure caused the extension to use `~/.pi/remote/identity.json`; that file contained a keypair divergent from the owner-paired key. The resulting new public key was absent from the Owner's peer list, and the self-revoke poller removed the pairing and detached the Owner channel. The `/new` command was incidental: identity loading occurred at relay/pairing startup and the poller was relay-lifecycle-owned. Recovery was `/outpost-pi pair`. The note identifies the resilience gap as silent destructive fallback and proposes write-through plus loud/fail-fast mismatch handling.

## Key passages

- "**PC:** `patchbay` (workstation, a sibling Pi in the operator's mesh)"
- "`KeyRevoked` — patchbay's platform keyring became inaccessible"
- "**BUT the file keypair had diverged from the key the owner app originally paired with**"
- "New pubkey → owner's peer list no longer contains patchbay → the **selfRevoke poller** ... found patchbay missing"
- "`/new` caused neither the keyring failure nor the re-identity nor the eviction."
- "The gap is in the identity-fallback path: either the file didn't mirror the keyring key (write-through missing), or the fallback minted/used a different key rather than failing fast."
- "**Silent re-identity on fallback is the bug.**"
- "`/outpost-pi pair` on patchbay re-pairs it with the owner app."
