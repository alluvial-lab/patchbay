---
source_handle: aider-gui
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/gui.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: browser UI path

## Core findings
- Browser mode is implemented via Streamlit (`import streamlit as st`), with a `GUI` class that reuses a single `Coder` state for web interactions.
- `main.py` routes `--gui`/`--browser` to `launch_gui`, which calls `streamlit.web.cli.main(['run', <gui module>, ...])`.
- GUI path is not a separate privileged agent supervisor API; it is a UI frontend layered on the same underlying command/`Coder` model.

## Evidence snippets
1) `launch_gui(args)` in main calls `streamlit.web.cli.main(st_args)`.
2) GUI state uses `@st.cache_resource` and `get_coder()` delegating to `cli_main(return_coder=True)`.
3) `CLI` docs mention browser option as alternative interface.