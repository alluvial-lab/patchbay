---
source_handle: vscode-remote-ssh
fetched: 2026-07-07
source_url: https://code.visualstudio.com/raw/docs/remote/ssh.md
provenance: source-direct
---

# Attestation: VS Code Remote SSH

## Structural metadata

- Publisher/site: Visual Studio Code documentation.
- Page title observed: Remote Development using SSH.
- Source kind: remote-development user documentation.

## Paraphrased summary

VS Code Remote - SSH lets a local VS Code client open a remote folder over SSH, install VS Code Server on the remote OS, run commands/extensions remotely, open folders/workspaces, forward ports, and open terminals on the remote host.

## Key passages

1. **Purpose.** The page says Remote - SSH lets the user open a remote folder on a remote machine, VM, or container with a running SSH server and then interact with files and folders on the remote filesystem. Source anchor: lines 1-5.

2. **Server and remote execution.** The page says no source code needs to be local because commands and extensions run directly on the remote machine; the extension installs VS Code Server on the remote OS independent of any existing VS Code installation there. Source anchor: lines 3-5.

3. **Connect setup feedback.** The getting-started sequence says VS Code connects to the SSH server, sets itself up, shows progress, and exposes detailed logs in the Remote - SSH output channel. Source anchor: lines 76-80.

4. **Remote folders/workspaces.** After connection, the user can open any folder or workspace on the remote machine through normal VS Code UI commands. Source anchor: lines 88-92.

5. **Port forwarding.** The page describes forwarding a remote port to the local machine using an SSH tunnel for ports not publicly exposed. Source anchor: lines 202-204.

6. **Remote terminal.** Any terminal window opened in VS Code while connected runs on the remote host rather than locally. Source anchor: lines 246-250.

7. **No built-in source sync/local tooling support.** The page says Remote - SSH does not directly support syncing source code or using local tools with content on a remote host, recommending common tools such as SSHFS or rsync. Source anchor: lines 266-272.
