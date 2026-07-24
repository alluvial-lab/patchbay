---
id: backlog-icon-set-adoption
created: 2026-07-23
tags: [ux, design-system, fast-follower]
research_origin: null
---

# Backlog: adopt an icon set for the cockpit chrome

Surfaced in live use (2026-07-23): the operator noted the composer's action
buttons should be icons, not text — a paperclip for Attach, an arrow for Send
("the typical paperclip button, and Send as a typical arrow button"). The
project currently has **no icon set** — `components.css` uses text labels plus
a few ad-hoc unicode glyphs and one inline SVG (the paperclip added in
`4a903cd` as a stopgap).

## Shape

- **Pick an icon set.** Open question the operator raised: "what do we use in
  SNC/platform?" — resolve which set that is, or choose a standard one for
  this console aesthetic (Lucide is the common modern fit; Heroicons and
  Phosphor are the usual alternatives). Decision needed from the operator.
- **Integrate it into the design system** (`.mockups/design-system/`), not ad
  hoc per-button: an icon primitive (`.icon`, size tokens, stroke conventions)
  in `components.css`, passing `check-presentation` (the conformance floor
  must bind any new primitive, not bypass it).
- **Apply it:** composer actions (Attach → paperclip, Send → arrow-up),
  sidebar header actions (spawn/attach → plus/link), back buttons, disclosure
  carets, the delivery-badge expand affordance, and the Cancel/Interrupt
  contextual actions.

## Notes

- The stopgap paperclip (`4a903cd`) is an inline SVG in `session-detail.ts`;
  replace it with the chosen set's primitive when this lands.
- Keep the single-file/no-build-step mockup convention in mind — an
  inline-SVG-sprite or inline-path approach fits better than an icon font or
  npm icon package with a build step.
