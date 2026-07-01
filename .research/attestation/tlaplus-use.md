---
source_handle: tlaplus-use
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/tlaplus/master/USE.md
provenance: source-direct
---

# Per-source attestation: tlaplus-use

## Paraphrased summary

The TLA+ Tools `USE.md` describes command-line use of `tla2tools.jar`, states the Java runtime requirement, shows classpath-based invocations for individual tools, and states that `java -jar tla2tools.jar` is an alias for `tlc2.TLC`.

## Key passages

[tlaplus-use]{1} The document says the TLA+ tools require Java 11 or newer to run.

[tlaplus-use]{2} For command-line use, the document says to get `tla2tools.jar` from GitHub releases and says the jar contains multiple TLA+ tools.

[tlaplus-use]{3} The classpath examples include `java tlc2.TLC -help` for the TLA+ model checker, alongside SANY, REPL, PlusCal translator, TLA-to-LaTeX, and XML exporter examples.

[tlaplus-use]{4} The document says: running `java -jar tla2tools.jar` is aliased to run `tlc2.TLC`.

[tlaplus-use]{5} For Java applications, the document says model-checking can call `tlc2.TLC.main()` with the same argument list as the command line, and warns that TLC cannot be run twice in the same process because it maintains global static state.

[tlaplus-use]{6} For non-Java applications, the document says to run Java on `tla2tools.jar` and capture process output. It also mentions state-space dumping with `-dump dot output.gv` and JSON trace dump/load flags in this master-branch document.

## Structural metadata

- Source type: repository documentation file.
- Repository branch fetched: `master`.
- Document title: `Using the TLA⁺ Tools`.
