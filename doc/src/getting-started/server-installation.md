# Server Installation

## Platform Requirements

MELISA supports all major Linux distributions. The `--setup` routine automatically detects your host OS and selects the appropriate package manager, firewall tool, and service names:

| Host Distribution | Package Manager | Firewall | SSH Service |
|-------------------|-----------------|----------|-------------|
| Fedora, RHEL, CentOS, Rocky Linux | `dnf` | `firewalld` | `sshd` |
| Ubuntu | `apt-get` | `ufw` | `ssh` |
| Debian | `apt-get` | `ufw` | `ssh` |
| Arch Linux | `pacman` | `iptables` | `sshd` |
| Other / Unknown | `apt-get` (fallback) | `ufw` (fallback) | `ssh` (fallback) |

Detection is performed by parsing `/etc/os-release` at setup time. If your distribution is not explicitly listed, MELISA falls back to `apt-get` defaults with a warning and will still attempt to complete setup.

The `--setup` routine installs and configures the following system components:

- `lxc`, `lxc-templates` / `lxc-utils` — Linux Container runtime and base images
- `bridge-utils` — Network bridge configuration tools
- `openssh-server` / `openssh` — SSH daemon for remote client access
- `firewalld` / `ufw` / `iptables` — Firewall management (selected automatically)

---

## Step 1: Compile from Source

MELISA has no pre-compiled binary distribution. You must build it from source using the Rust compiler. This ensures the binary is optimized for your hardware.

**Install Rust** (if you don't have it):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Clone and build MELISA:**

```bash
git clone https://github.com/ernoba/melisa-on-saferoom-project.git
cd melisa-on-saferoom-project
cargo build
```

The compiled binary will be at `./target/debug/melisa`. The build process uses the following release profile for production builds:

```toml
[profile.release]
opt-level = 'z'      # Optimize for binary size
lto = true           # Link Time Optimization: removes dead code
codegen-units = 1    # Single code generation unit for maximum optimization
panic = 'abort'      # Remove unwinding logic on panic
strip = true         # Strip debug symbols from the final binary
```

---

## Step 2: First Launch

Run the compiled binary with elevated privileges. The `-E` flag preserves your environment variables (important for `SUDO_USER` detection):

```bash
sudo -E ./target/debug/melisa
```

If successful, you will see the animated boot sequence followed by the MELISA dashboard:

```
  >> INITIALIZING CORE ENGINE...
  0x4A2F1C8E [ OK ] Initializing core subsystem...
  0x7B3E9D1A [ OK ] Verifying LXC bridge connectivity...
  0x1F6C4B2D [ OK ] Loading security namespaces...
  ...

  [ DONE ] KERNEL INITIALIZED: M.E.L.I.S.A // SYSTEM_STABLE_ENVIRONMENT

 ███╗   ███╗███████╗██║     ██║███████╗███████╗
 ...
     [ MANAGEMENT ENVIRONMENT LINUX SANDBOX ]

  ┌─── SYSTEM TELEMETRY & STATUS ──────────────────────────────────────┐
  │ TIMESTAMP  :: 2026-03-20 16:25:23
  │ KERNEL_ID  :: FEDORA LINUX
  │ HOST_NODE  :: YOUR_HOSTNAME
  │ PROCESSOR  :: Your CPU Model
  │ GPU_STATUS :: Your GPU
  │ RAM_USAGE  :: XXXMB / XXXMB (XX%)
  │ ------------------------------------------------------------------
  │ PROTOCOL   :: SECURE ISOLATION ACTIVE
  │ DIRECTIVE  :: MAXIMUM PERFORMANCE // ZERO INEFFICIENCY
  └────────────────────────────────────────────────────────────────────┘

  >>> ALL SYSTEMS OPERATIONAL. SECURE SESSION GRANTED.
  ENTER COMMAND:
melisa@yourhostname:~>
```

---

## Step 3: The Physical Handshake — Running `--setup`

> **⚠️ Critical Security Requirement**
>
> The `--setup` command **must be executed on a physical terminal session**. It explicitly detects and refuses SSH connections. This is a deliberate security design: an attacker who gains network access before your defenses are configured cannot remotely bootstrap the system. This is a **one-time operation**.

At the MELISA prompt, run:

```
melisa@yourhostname:~> melisa --setup
```

### What `--setup` Does (In Order)

The installation routine performs the following steps, each with timeout protection and status reporting:

| Step | Action | Details |
|------|--------|---------|
| 1 | **Distro Detection** | Reads `/etc/os-release` and selects the correct package manager, firewall, and service names |
| 2 | **System Update** | Updates package repositories (`dnf update` / `apt-get update` / `pacman -Sy`) |
| 3 | **Dependency Installation** | Installs `lxc`, bridge tools, `openssh-server`, and firewall package |
| 4 | **Kernel Module Loading** | Loads `veth` kernel module for virtual network interface pairs |
| 5 | **Service Activation** | Enables and starts `lxc.service`, `lxc-net.service`, SSH daemon, and firewall |
| 6 | **Binary Deployment** | Copies the MELISA binary to `/usr/local/bin/melisa` with **SUID bit (4755)** |
| 7 | **Shell Registration** | Adds `/usr/local/bin/melisa` to `/etc/shells` as a valid login shell |
| 8 | **Global Sudoers Rule** | Creates `/etc/sudoers.d/melisa` — allows all users to run `melisa` without a password |
| 9 | **Projects Directory** | Creates `/opt/melisa/projects` with Sticky Bit (`chmod 1777`) |
| 10 | **Firewall Configuration** | Opens SSH port; trusts `lxcbr0` bridge interface (supports `firewalld`, `ufw`, `iptables`) |
| 11 | **LXC Network Quota** | Configures `/etc/lxc/lxc-usernet` — grants the executing user permission to manage virtual ethernet interfaces |
| 12 | **User Namespace Mapping** | Sets `subuid`/`subgid` mappings (`100000–165535`) via `usermod` for unprivileged container support |
| 13 | **SUID Fix** | Sets SUID on `/usr/bin/newuidmap` and `/usr/bin/newgidmap` for user namespace traversal |
| 14 | **Privacy Hardening** | `chmod 711 /home` — prevents users from listing other users' home directories |
| 15 | **Git Security** | Configures `git config --system safe.directory '*'` — prevents "dubious ownership" errors across user boundaries |

### Firewall Detection

`--setup` automatically detects the active firewall and configures it accordingly:

```
firewalld  →  firewall-cmd (SSH zone + lxcbr0 trusted)
ufw        →  ufw allow ssh + ufw allow in on lxcbr0
iptables   →  iptables -A INPUT rules for port 22 and lxcbr0
```

---

## Step 4: Verification

Once setup completes, exit and re-enter MELISA to confirm the installation:

```bash
melisa --list
```

An empty list (no containers yet) is the expected successful output:

```
[INFO] Retrieving container inventory...
NAME   STATE   AUTOSTART   GROUPS   IPV4   IPV6   UNPRIVILEGED
```

Your host is now a fully operational MELISA Orchestration Node. 🎉

---

## System File Locations

After a successful `--setup`, these files and directories are created or modified:

| Path | Purpose |
|------|---------|
| `/usr/local/bin/melisa` | Main MELISA binary (SUID, mode 4755) |
| `/etc/shells` | Contains `/usr/local/bin/melisa` as a valid login shell |
| `/etc/sudoers.d/melisa` | Global passwordless sudo rule for the `melisa` binary |
| `/opt/melisa/projects/` | Master bare Git repository storage (mode 1777) |
| `/var/lib/lxc/` | LXC container rootfs and configuration storage |
| `/etc/lxc/lxc-usernet` | Per-user network interface quota |