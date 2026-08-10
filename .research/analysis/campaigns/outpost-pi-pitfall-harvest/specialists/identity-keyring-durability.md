---
source_handle: identity-keyring-durability
campaign: outpost-pi-pitfall-harvest
facet: identity-keyring-durability
updated: 2026-08-09
provenance: agent-synthesis
---

# Identity / keyring durability: silent re-identity

## Scope and source boundary

This facet is about the `outpost_pi` incident recorded on the `patchbay` workstation. The source identifies `patchbay` as the affected PC and the source repository as the Outpost-Pi implementation; it does **not** identify Patchbay (this repository) as the component that lost the credential. `[keyring-incident]{1}`

## Disconfirming analysis

The observed `/new` command is a tempting causal explanation, but the incident source explicitly separates session lifecycle from relay/pairing identity loading and says `/new` caused neither the keyring failure nor the re-identity nor the eviction. `[keyring-incident]{5}` The keyring failure's exact platform cause is also unresolved: the source lists an inaccessible, locked, gone, or unfindable entry and gives the rebrand service rename only as a possibility. `{ambiguous}` `[keyring-incident]{2}`

## Failure chain

1. At relay/pairing startup, the PC's platform keyring returned `KeyRevoked`. The extension fell through to `~/.pi/remote/identity.json`. `[keyring-incident]{2}`
2. The file-held keypair differed from the key the Owner had originally paired. The process therefore presented a different public key: a silent local principal change rather than a declared pairing or rotation. `{inferred}` `[keyring-incident]{3}`
3. The Owner's signed membership view no longer listed that public key. The relay-side membership model therefore treated the PC as absent, and the extension's periodic self-revoke poller detected the absence. `[keyring-incident]{4}`
4. The poller removed the Owner peer and invoked the revoke callback. The callback detached the live Owner channel with `session_replaced`, refreshed pairing state, and emitted a `mesh-revoked` message directing the operator to re-pair. `[keyring-self-revoke]{2}` `[keyring-self-revoke]{5}`
5. Re-pairing was the recorded recovery. `[keyring-incident]{8}`

The immediate containment was therefore owner-mesh eviction, not restoration of the original identity. It stopped the old pairing from being treated as live, but required an operator re-pair to establish a new authorized binding. `{inferred}` `[keyring-self-revoke]{5}`

## Why this is an authority failure

Outpost-Pi's recorded trust model makes identity the public key, uses Ed25519 challenge-response, persists pairings, and assigns one Pi key to a PC; hardware change is explicitly a re-pairing event. `[keyring-decisions]{1}` `[keyring-decisions]{2}` `[keyring-decisions]{3}` `[keyring-decisions]{4}`

Under that model, credential loss must not be interpreted as permission to adopt an unrelated local key. The incident crossed an authority boundary without an explicit transition: the same physical PC silently became a new cryptographic principal, while the operator received no identity-continuity decision before the Owner membership check later evicted it. `{inferred}` `[keyring-incident]{3}` `[keyring-incident]{7}` This is the relevant Patchbay pitfall: **credential unavailability is not identity absence, and a fallback key is not automatically the same authority.** `{inferred}`

The signed membership and self-revoke path did the right thing for the key it observed: membership is Owner-authenticated, and the relay is not the membership adjudicator. `[keyring-decisions]{5}` `[keyring-self-revoke]{1}` The failure was earlier, at local identity resolution: a storage recovery path changed the key presented to that authority system without making the change explicit. `{inferred}`

## Root cause and remediation status

### Disconfirming analysis

The repository contains meaningful guards against one variant of silent rotation. The current storage implementation retries keyring reads; on a core-keyring platform it throws `KeyringUnavailableError` when the keyring remains unreadable and no file identity exists; and its tests assert that this path does not regenerate a file identity. `[keyring-storage]{2}` `[keyring-storage]{5}` `[keyring-tests]{2}` These guards support the narrower claim that "persistent keyring failure with no fallback file" is now loud on the guarded platforms, not the broader claim that identity continuity is solved.

### Load-bearing finding

The remaining root cause is **unverified fallback continuity**. The storage API returns any parseable file identity when keyring reads fail; the source records no comparison between that file public key and the last known paired local public key. `{inferred}` `[keyring-storage]{1}` `[keyring-storage]{4}` The incident's divergent file key is exactly the case this rule permits. `[keyring-incident]{3}`

The mobile Owner identity path documents the analogous safer boundary: generate only after a successful null load, surface store errors, and re-read before saving so a late restored identity wins. `[keyring-identity-read-remediation]{3}` `[keyring-identity-read-remediation]{4}` `[keyring-identity-read-remediation]{5}` This is a directly reusable seam for Patchbay's Pi identity path, though it does not itself solve the file/keyring mismatch. `{inferred}`

There is also a write-through gap. The keyring-success path reads an existing key or generates and writes a keyring value, but does not write the same keypair to `identity.json`; the file writer is used only when the fallback itself generates a fresh file identity. `[keyring-storage]{6}` The storage tests explicitly expect no file after a successful retry to the original keyring entry. `[keyring-tests]{1}` Thus the implementation and test suite do not establish the session note's intended invariant that the file mirrors the keyring key. `{inferred}` `[keyring-incident]{6}`

The practical distinction for Patchbay is:

- **Already addressed in the source:** transient/core-keyring failure should be retried or surfaced rather than immediately minting a new key. `[keyring-storage]{2}` `[keyring-tests]{1}` `[keyring-tests]{2}`
- **Still open as a continuity seam:** if a file exists but is stale, divergent, or merely unproven, returning it is still silent re-identity; and a keyring-created identity is not currently guaranteed to be mirrored. `{inferred}` `[keyring-storage]{4}` `[keyring-storage]{6}`
- **Containment, not continuity repair:** self-revoke removes the mismatched Owner pairing and tells the operator to re-pair. `[keyring-self-revoke]{2}` `[keyring-self-revoke]{5}`

## Patchbay pitfall and seam decisions

### Disconfirming analysis

A file mirror alone is not sufficient: the incident demonstrates that a file can exist and still diverge from the Owner-paired key. `[keyring-incident]{3}` Conversely, always refusing to operate whenever a platform keyring is unavailable would make documented headless/file-only operation impossible; the source explicitly supports a file fallback on platforms without a guaranteed core keyring. `[keyring-storage]{2}` The design must therefore distinguish continuity-preserving recovery from authorized identity creation, rather than choosing a universal "always fallback" or "always fail" rule. `{inferred}`

### Decision: make continuity a state transition, not a storage accident

Patchbay's authority model should reserve these distinct outcomes: `{inferred}`

- `identity_loaded`: the durable key and its expected identity binding were recovered.
- `credential_unavailable`: the keyring could not be read; no principal substitution occurs.
- `identity_continuity_verified`: a fallback copy is accepted only after matching a durable last-known public-key binding (and, where available, an Owner-paired binding).
- `identity_continuity_unknown`: the keyring is unavailable and the available file key cannot be proven to be the prior principal; block authority-bearing startup and surface a recovery action.
- `identity_rotated_authorized`: a new key is created only through an explicit operator-approved pairing/rotation ceremony, with old/new fingerprints and authority provenance recorded.

The invariant to carry into the authority/descendant-grant work is: **`credential_loss -> continuity_verified | loud_recovery_required`; never `credential_loss -> silent_new_principal`.** `{inferred}` `[keyring-incident]{7}` `[keyring-decisions]{2}`

### Decision: keep credential rotation separate from identity continuity

A credential/keyring backend may rotate, become unavailable, or be migrated without implying a principal change. `{inferred}` The durable identity record should bind the public key (or a stable identity identifier derived from it), its authority domain, and the provenance of any transition; keyring replacement is a storage event, while a public-key change is an authority event. `{inferred}` This follows the source's identity-as-pubkey and one-Pi-key-per-PC decisions. `[keyring-decisions]{2}` `[keyring-decisions]{4}`

### Decision: fail before presenting an unproven identity

On a continuity mismatch or unknown continuity, Patchbay should not start relay authority, issue descendant grants, or participate as the old principal. `{inferred}` It should emit a durable, operator-visible event containing the failure reason, prior-known fingerprint if available, candidate fingerprint only as diagnostic data, and an explicit recovery path. `{inferred}` The Outpost-Pi remediation message is a useful operational shape—loud explanation plus re-pair—but Patchbay should distinguish "identity continuity lost" from an Owner intentionally revoking a still-valid identity. `{inferred}` `[keyring-self-revoke]{5}`

### Decision: treat owner-mesh eviction as defense in depth

Signed Owner membership and eviction remain valuable containment if an unexpected key reaches the mesh. `[keyring-decisions]{5}` `[keyring-self-revoke]{1}` They must not be the primary continuity mechanism: by the time the poller evicts, the process has already presented the wrong principal and the operator has already lost continuity. `{inferred}` The primary gate belongs at identity resolution/startup; the mesh eviction is the secondary authority check.

## Contradictions

| Source-side statement | Counter-evidence | Consequence |
|---|---|---|
| The incident's intended storage behavior was transparent fallback through a file that mirrors the same keypair. `[keyring-incident]{6}` | The current keyring-success path does not write the file mirror, and the retry test asserts that no file is written. `[keyring-storage]{6}` `[keyring-tests]{1}` | The mirror invariant is not implemented or tested; divergence remains possible. `{inferred}` |
| Pairings are persistent and a PC has one stable Pi key; key changes require re-pairing. `[keyring-decisions]{1}` `[keyring-decisions]{4}` | On keyring failure, a present file key is accepted without a known-key comparison. `[keyring-storage]{4}` | A local storage error can emulate a key change without the explicit re-pair ceremony. `{inferred}` |
| Core-keyring failure with no file is now fail-loud. `[keyring-storage]{2}` `[keyring-tests]{2}` | Core-keyring failure with a file takes the file path; the source does not prove that file's continuity. `[keyring-storage]{4}` | The remediation closes only the no-file branch, not stale/divergent-file recovery. `{inferred}` |

## Gaps and verification targets

### Disconfirming analysis

The primary incident note leaves the underlying `KeyRevoked` cause open and explicitly requests comparing the file public key with the Owner app's stored patchbay key and inspecting whether the renamed keyring entry exists, is locked, or is gone. `[keyring-incident]{2}` The findings below therefore separate observed facts from unresolved diagnostics.

### Gaps

1. **Cause attribution gap:** no source confirms whether the local keyring entry was revoked, locked, deleted, or lost during the destructive service rename. `{ambiguous}` `[keyring-incident]{2}`
2. **Continuity-proof gap:** the storage code and tests do not compare fallback identity against a last-known local/Owner binding. `{inferred}` `[keyring-storage]{4}` `[keyring-tests]{3}`
3. **Write-through gap:** no source-backed test proves keyring mint/read and file mirror remain identical across restart and backend loss. `{inferred}` `[keyring-storage]{6}` `[keyring-tests]{1}`
4. **Transition/audit gap:** the observed user-facing event was revocation after membership reconciliation, not a prior identity-continuity-loss event. `{inferred}` `[keyring-incident]{4}` `[keyring-self-revoke]{5}`
5. **Formal-model gap for Patchbay:** the failure should become a checked transition invariant and conformance vectors covering keyring read error, stale file, explicit rotation, descendant authority, and owner-mesh eviction. `{inferred}` The local source establishes the need for those cases but does not supply Patchbay's model. `[keyring-incident]{7}` `[keyring-decisions]{4}`

## Suggested Patchbay acceptance vectors

These are design proposals, marked `{inferred}` rather than claims about Outpost-Pi: `{inferred}`

- A keyring read error never creates or adopts a new authority without explicit operator authorization.
- A fallback file is accepted only when its public key matches the durable expected identity binding; otherwise startup returns `identity_continuity_unknown` and emits an operator-visible diagnostic.
- Keyring success writes/validates the mirror atomically, and restart/backend-loss tests prove byte-identical key material.
- Explicit credential rotation records old identity, new identity, actor/authority, reason, and descendant-grant consequences; it is not represented as ordinary recovery.
- A process that cannot prove identity continuity cannot mint, forward, or accept descendant authority-bearing operations.
- Owner-mesh eviction remains an independently verified containment path and is distinguishable from local identity-continuity failure in the audit/event vocabulary.

## Revisit if

- The open patchbay diagnostic identifies the exact keyring failure and whether the file key matched any historical owner binding. `[keyring-incident]{2}`
- Patchbay's identity/keyring implementation is introduced; then replace these source-derived proposals with implementation-specific vectors and a checked model.
- Credential rotation, device migration, or recovery export becomes a committed Patchbay capability; the continuity-versus-rotation seam must be reopened rather than silently extending the fallback behavior. `{inferred}` `[keyring-decisions]{4}`
