# Chapter I: The Foundation

*Or: How to perform a ritual that only you can perform.*

---

Erick has done this before. Not with MELISA — MELISA didn't exist yet — but with the memory of every late Friday night when a developer messaged him saying *"I think I broke something"* and he would open a remote session to find a laptop with seventeen conflicting Python versions, a corrupted package registry, and three different versions of Node living in places Node was never meant to live.

He built MELISA to end that.

Tonight he's alone in the server room. A Fedora Linux machine sits in front of him. It has internet. It has a monitor and a physical keyboard. These two things matter — he'll explain why in a moment.

---

## The Prerequisite: Being There

He pulls up the repository on his phone and reads the first requirement aloud to no one in particular:

> *Physical/Console Access: For security reasons, the setup command refuses to run over SSH.*

This is his rule. He wrote it into the code himself. The reasoning is simple: if an attacker compromises your network before you've even finished setting up your defenses, you don't want them to be able to remotely configure your server. The moment of initialization is the most vulnerable. So MELISA demands that you be *present* for it.

He connects the keyboard, opens a terminal, and installs Rust.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Then:

```bash
git clone https://github.com/ernoba/melisa-on-saferoom-project.git
cd melisa-on-saferoom-project
cargo build
```

The compiler runs. Rust is a slow compiler when you watch it and instantaneous when you don't, but tonight Erick watches. Dependencies resolve. The binary materializes.

---

## The First Boot

```bash
sudo -E ./target/debug/melisa
```

The screen clears. Then:

```
  >> INITIALIZING CORE ENGINE...
  0x4A2F1C8E [ OK ] Initializing core subsystem...
  0x7B3E9D1A [ OK ] Verifying LXC bridge connectivity...
  0x1F6C4B2D [ OK ] Loading security namespaces...
  0x9E2A3F87 [ OK ] Syncing environment variables...
  0x3C8D1E56 [ OK ] Establishing encrypted session...

  [ PROC ] DECRYPTING KERNEL: X#?!X#?!X#?!...
  [ DONE ] KERNEL INITIALIZED: M.E.L.I.S.A // SYSTEM_STABLE_ENVIRONMENT
```

He lets himself enjoy this for exactly three seconds. The animation took him longer than the actual logic, and he refuses to feel bad about that.

The screen clears again and the dashboard appears:

```
 ███╗   ███╗███████╗██║     ██║███████╗███████╗
 ████╗ ████║██╔════╝██║     ██║██╔════╝██╔══██╗
 ██╔████╔██║█████╗  ██║     ██║███████╗███████║
 ██║╚██╔╝██║██╔══╝  ██║     ██║╚════██║██╔══██║
 ██║ ╚═╝ ██║███████╗███████╗██║███████║██║  ██║
 ╚═╝     ╚═╝╚══════╝╚══════╝╚═╝╚══════╝╚═╝  ╚═╝
     [ MANAGEMENT ENVIRONMENT LINUX SANDBOX ]

  ┌─── SYSTEM TELEMETRY & STATUS ──────────────────────────────────────┐
  │ TIMESTAMP  :: 2026-03-20 20:14:33
  │ KERNEL_ID  :: FEDORA LINUX
  │ HOST_NODE  :: SAFEROOM-01
  │ PROCESSOR  :: AMD Ryzen 7 7435HS
  │ GPU_STATUS :: NVIDIA Corporation GA107
  │ RAM_USAGE  :: 2048MB / 16384MB (12%)
  │ ------------------------------------------------------------------
  │ PROTOCOL   :: SECURE ISOLATION ACTIVE
  │ DIRECTIVE  :: MAXIMUM PERFORMANCE // ZERO INEFFICIENCY
  └────────────────────────────────────────────────────────────────────┘

  >>> ALL SYSTEMS OPERATIONAL. SECURE SESSION GRANTED.
  ENTER COMMAND:
melisa@saferoom-01:~>
```

MELISA is alive. But it hasn't been initialized yet. This is just the engine idling.

---

## The Physical Handshake

Erick types the command that kicks off everything:

```
melisa@saferoom-01:~> melisa --setup
```

Fifteen things happen in sequence. He watches the status lines appear one by one:

```
Initializing MELISA Host Environment...

System Update & Dependencies...
  Updating system packages                  [ OK ]
  Installing lxc, lxc-templates             [ OK ]
  Installing libvirt, bridge-utils          [ OK ]
  Installing openssh-server, firewalld      [ OK ]

Kernel & Service Configuration...
  Loading veth kernel module                [ OK ]
  Enabling lxc.service                      [ OK ]
  Starting sshd                             [ OK ]
  Starting firewalld                        [ OK ]

Binary Deployment...
  Copying binary to /usr/local/bin/melisa   [ OK ]
  Applying SUID bit (4755)                  [ OK ]

...

System Privacy Hardening...
  Protecting /home directory indexing       [ OK ]
```

The last line disappears. A summary appears. Everything succeeded.

He looks at the machine. It's a different machine now. Before `--setup`, it was a server running Fedora with some software installed. After `--setup`, it's an orchestration node — a platform that can carve isolated environments out of the Linux kernel on demand, manage users with surgical precision, and be controlled from anywhere in the world over SSH.

He closes the terminal. His physical presence is no longer required.

---

## What Just Happened (The Dry Version)

For those who want the facts:

`--setup` installed LXC, configured the `lxcbr0` network bridge, deployed the MELISA binary to `/usr/local/bin/` with SUID permissions (mode `4755`), registered it as a valid login shell in `/etc/shells`, set up passwordless sudo rules in `/etc/sudoers.d/melisa`, created `/opt/melisa/projects/` as the master repository storage, configured the firewall to allow SSH and trust the LXC bridge, mapped user namespaces (`subuid`/`subgid`), and hardened `/home` to prevent user enumeration.

The binary at `/usr/local/bin/melisa` now has the SUID bit set. When any system user runs it, it executes as root — but only MELISA's own controlled code runs with those privileges.

The server is ready.

**Next:** [Chapter II — The First Saferoom](./chapter-2-the-first-saferoom.md)