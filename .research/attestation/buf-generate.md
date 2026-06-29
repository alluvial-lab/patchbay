---
source_handle: buf-generate
fetched: 2026-06-28
source_url: https://buf.build/docs/generate/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Buf code generation docs

## Summary

Buf documents `buf generate` as a Protobuf code-generation command that runs plugins over `.proto` inputs and emits code in target languages including TypeScript and Rust. Generation is configured by `buf.gen.yaml`. Buf can use remote plugins pinned in configuration, local plugins, and built-in `protoc` plugins. The docs also describe managed mode as a way to move language-specific options out of `.proto` files and into generation configuration.

## Key passages

1. From the "Generating code" page introduction:

> `buf generate` runs Protobuf plugins over your `.proto` files, producing source code in whatever language the plugins target: Go, TypeScript, Java, Python, C++, Rust, and so on. It reads a `buf.gen.yaml` file that lists the plugins to run and where to put their output, then runs them.

2. From "How it compares to protoc":

> Everything lives in `buf.gen.yaml` instead of on the command line. Check the file in, and every developer and CI run produces the same output.

3. From "How it compares to protoc" and "Remote plugins":

> Plugins don’t need to be installed locally. The BSR hosts remote plugins pinned by version, so builds are reproducible without a separate install step.

4. From "Managed mode":

> Managed mode moves language-specific options like `go_package`, `java_package`, or `csharp_namespace` out of your `.proto` files and into `buf.gen.yaml`. Your schema stays language-neutral, and each consumer can generate code their own way without editing the source files.
