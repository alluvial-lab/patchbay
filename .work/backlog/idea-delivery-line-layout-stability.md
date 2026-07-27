---
id: idea-delivery-line-layout-stability
tags: [ui, cockpit, idea]
created: 2026-07-27
---

# Delivery/status line: layout shift + separate-box noise

Dogfooding report (2026-07-27): the command status bar rendered below an
operator instruction bounces around as the command changes state — suspected
cause: the interrupt/stop action appears and disappears with the
running/terminal transition, changing the bar's size and reflowing the
timeline. Also: like the tool-call preview before its fix, the delivery line
sits in a SEPARATE box below the instruction message, which reads as visual
noise.

Direction for the UX batch: fold the delivery state into the instruction
card (same treatment as the tool-call preview), reserve stable space (or a
stable-dimension affordance) for the interrupt action so state transitions
don't shift layout, and spec the state-transition visual in the mockup pass.

Include in the cockpit UX batch alongside session-list row redesign and the
settings section.
