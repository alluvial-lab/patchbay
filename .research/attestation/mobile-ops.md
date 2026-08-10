---
source_handle: mobile-ops
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/features/feature-mobile-slash-command-invocation.md (commit 32dd3a6); cockpit/lib/app/cockpit/ui/widgets/agent_composer.dart
provenance: source-direct
---

# Source summary

The source records the rescope from arbitrary slash-command invocation to dedicated operations, and the cockpit implementation routes built-ins through dedicated RPC-backed methods.

## Key passages

{1} > The real Pi-ecosystem pattern: dedicated operations per command (the cockpit routes `/new`+`/compact` through dedicated RPC).

{2} > The SDK gates session-control (`newSession`/`fork`/`switch`/`reload`) behind `ExtensionCommandContext` — *"only safe in user-initiated commands"* — while safe ops (`compact`/`abort`/`shutdown`) sit on the base `ExtensionContext`.

{3} > That's why `compact` works from mobile but `/new` doesn't: `compact()` is base-context; `newSession()` is command-gated by design. No general command-invoke API exists.

Cockpit implementation excerpt:

{4} > `// Route a pure built-in (/new, /compact) through its dedicated RPC. A command with attachments is sent as a prompt.`

{5} > `case 'new': ownAsync(widget.session.startNewSession());`

{6} > `case 'compact': ownAsync(widget.session.compact());`

## Metadata

- Repository: `/home/agent/projects/outpost_pi`
- Commit/work source: `32dd3a6`
- Code source: `cockpit/lib/app/cockpit/ui/widgets/agent_composer.dart`
- Source type: local repository design record and implementation
