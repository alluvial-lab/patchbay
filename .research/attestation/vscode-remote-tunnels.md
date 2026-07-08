---
source_handle: vscode-remote-tunnels
fetched: 2026-07-07
source_url: https://code.visualstudio.com/raw/docs/remote/tunnels.md
provenance: source-direct
---

# Attestation: VS Code Remote Tunnels

## Structural metadata

- Publisher/site: Visual Studio Code documentation.
- Page title observed: Developing with Remote Tunnels.
- Source kind: remote-development user documentation.

## Paraphrased summary

VS Code Remote Tunnels lets a VS Code client connect to a remote machine through a secure tunnel without SSH. The remote side runs VS Code Server, clients attach through the tunnel, and the documented controls include starting/stopping/unregistering tunnels and connecting to active tunnel machines. The model is a remote IDE connection to a machine, not a semantic operation lifecycle for application commands.

## Key passages

1. **Purpose.** The page says Remote - Tunnels lets the user connect to a remote machine such as a desktop PC or VM via secure tunnel, from a VS Code client anywhere, without SSH. Source anchor: lines 1-5.

2. **CLI starts the server and tunnel.** The getting-started section says `code tunnel` creates a secure tunnel and downloads/starts VS Code Server on the machine. Source anchor: lines 48-56.

3. **Reachability tied to remote VS Code/tunnel process.** The page notes that the remote machine is reachable through a tunnel only while VS Code remains running there, unless VS Code is started there again or `code tunnel` is run. Source anchor: lines 74-76.

4. **Connect to active tunnels.** The Remote Tunnels extension command can connect to any remote machines with an active tunnel, and the Remote Explorer can show remote machines. Source anchor: lines 80-88.

5. **Stop/unregister controls.** The FAQ says `Ctrl+C` stops a CLI tunnel, a VS Code command can turn off remote tunnel access, and `code tunnel unregister` removes a machine's association with tunneling. Source anchor: lines 129-131.

6. **Security transport.** Hosting and connecting to a tunnel require authentication with the same GitHub or Microsoft account; VS Code makes outbound connections to Azure-hosted service and does not generally require firewall listeners; after connect, an SSH connection is created over the tunnel for end-to-end encryption. Source anchor: lines 133-139.

7. **Service mode.** The page says `code tunnel service install` / `uninstall` can run the tunnel as a service, and `--no-sleep` can prevent the remote machine from sleeping. Source anchor: lines 155-160.
