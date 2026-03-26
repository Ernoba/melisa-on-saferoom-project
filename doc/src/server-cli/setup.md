# System Setup Commands

## `--setup`

**Access:** Administrator only · **Requires:** Physical terminal (non-SSH)

Initializes the MELISA host environment. This is a one-time bootstrapping operation that must be performed on a physical console.

```
melisa@host:~> melisa --setup
```

### Execution Flow

The setup routine runs 15 distinct phases, each with a 10–60 second timeout:

```
Initializing MELISA Host Environment...

System Update & Dependencies...
  Updating system packages (dnf update)     [ OK ]
  Installing lxc, lxc-templates             [ OK ]
  Installing libvirt, bridge-utils          [ OK ]
  Installing openssh-server, firewalld      [ OK ]

Kernel & Service Configuration...
  Loading veth kernel module                [ OK ]
  Enabling lxc.service                      [ OK ]
  Enabling lxc-net.service                  [ OK ]
  Starting sshd                             [ OK ]
  Starting firewalld                        [ OK ]

Binary Deployment...
  Copying binary to /usr/local/bin/melisa   [ OK ]
  Applying SUID bit (4755)                  [ OK ]

Shell Registration...
  Registering shell in /etc/shells          [ OK ]

Sudoers Access...
  Deploying global sudoers rule             [ OK ]
  Applying strict permissions (0440)        [ OK ]

Master Projects Infrastructure...
  Creating /opt/melisa/projects/            [ OK ]
  Setting Sticky Bit (1777)                 [ OK ]

Firewall Configuration...
  Opening SSH port (zone: public)           [ OK ]
  Assigning lxcbr0 to trusted zone          [ OK ]
  Reloading firewall                        [ OK ]

LXC Network Quota...
  Mapping SubUID/SubGID for <user>          [ OK ]

System Traversal Permissions...
  Setting SUID on /usr/bin/newuidmap        [ OK ]
  Setting SUID on /usr/bin/newgidmap        [ OK ]

System Privacy Hardening...
  Protecting /home directory indexing       [ OK ]

Global Git Security...
  Setting global git safe.directory='*'     [ OK ]
```

### Error Handling

Each step uses `timeout()` with a per-step deadline. If a step times out or fails:
- A `[FAILED]` or `[TIMEOUT]` status is printed
- Setup **continues** with remaining steps (non-fatal failures)
- A backup of modified system files is created before changes (e.g., `/etc/shells.bak`)

### When to Re-run Setup

You generally don't need to re-run `--setup`. However, it's safe to do so after:
- A major OS upgrade that resets service states
- Recovery from a system failure that took down `lxc-net.service`
- Adding a new user who needs UID mapping

---

## `--version`

**Access:** All users

Prints the MELISA version string and author information sourced from `Cargo.toml` at compile time:

```
melisa@host:~> melisa --version
MELISA Engine v0.1.2
Copyright (c) 2026 Erick Adriano Sebastian <ernobaproject@gmail.com>
```

---

## `--clear`

**Access:** Administrator only

Purges the MELISA command history both from memory and from disk:

```
melisa@host:~> melisa --clear
[SUCCESS] Local history file has been permanently deleted.
[DONE] Command history purge sequence completed successfully.
```

### Implementation Details

The history purge is deliberately TOCTOU-safe:

1. Clears the in-memory `rustyline` history buffer
2. Deletes the physical file (`~/.melisa_history`) — does not check existence first to avoid race conditions
3. Re-initializes an empty history file to keep `rustyline` from crashing on exit
4. Sets **mode `0600`** (owner read/write only) on the new empty file

The `0600` permission ensures no other user on the system can read command history that might contain sensitive container names, paths, or credentials.

---

## `--help` / `-h`

**Access:** All users (content is role-aware)

Displays the MELISA help manual. The output adapts to the calling user's role:

- **Standard users** see: General commands only
- **Administrators** see: General commands + Administration + Identity & Access Management + Project Orchestration sections

```
melisa@host:~> melisa --help

MELISA CONTROL INTERFACE - VERSION 0.1.2
Usage: melisa [options]

GENERAL COMMANDS
  --help, -h             Display this comprehensive help manual
  --version              Display system version and project metadata
  ...

ADMINISTRATION & INFRASTRUCTURE        ← (Admin only)
  --setup                Execute host-level environment initialization
  ...
```