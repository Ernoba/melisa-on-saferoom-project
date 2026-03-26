# Containers & Isolation

## What is an LXC Container?

MELISA uses **Linux Containers (LXC)** as its isolation layer. Unlike virtual machines, LXC containers share the host kernel but run in isolated namespaces for processes, networking, filesystems, and user IDs. This gives you:

- **Near-native performance** — no hypervisor overhead
- **Minimal disk footprint** — no full OS kernel per container
- **Fast provisioning** — containers start in seconds, not minutes
- **Complete isolation** — processes in a container cannot see or affect the host or other containers

## Container Anatomy

Every MELISA container lives at `/var/lib/lxc/<name>/` on the host:

```
/var/lib/lxc/mybox/
├── config                  ← LXC configuration (network, mounts, limits)
└── rootfs/                 ← The container's complete filesystem
    ├── etc/
    │   ├── melisa-info     ← MELISA metadata file (injected on creation)
    │   └── resolv.conf     ← DNS config (locked with chattr +i)
    ├── home/
    ├── usr/
    └── ...
```

## The `melisa-info` File

When MELISA creates a container, it injects a metadata file at `/etc/melisa-info` inside the rootfs. This file is written atomically (write to temp → set permissions → rename) to prevent corruption:

```
MELISA_INSTANCE_NAME=mybox
MELISA_INSTANCE_ID=f47ac10b-58cc-4372-a567-0e02b2c3d479
DISTRO_SLUG=ubuntu-jammy-amd64
DISTRO_NAME=ubuntu
DISTRO_RELEASE=jammy
ARCHITECTURE=amd64
CREATED_AT=2026-03-20T16:30:00+07:00
```

The `MELISA_INSTANCE_ID` is a UUID v4 generated at creation time. `--info <name>` reads this file.

## Networking

MELISA containers use the `lxcbr0` network bridge for connectivity:

```
Host Network Interface (eth0 / ens3)
        │
   ┌────┴────┐
   │ lxcbr0  │  ← Linux Bridge (created by lxc-net.service)
   └────┬────┘    IP range: 10.0.3.0/24 (default LXC)
        │
   ┌────┴────┐
   │ veth    │  ← Virtual Ethernet pair (one per container)
   └────┬────┘
        │
   Container (gets DHCP IP from lxcbr0's dnsmasq)
```

The `--setup` routine adds `lxcbr0` to the firewall's trusted zone, allowing container-to-host and container-to-internet traffic.

## DNS Locking

A subtle but important detail: after writing `/etc/resolv.conf` inside the container, MELISA runs:

```bash
chattr +i /etc/resolv.conf
```

This sets the **immutable flag**, preventing `systemd-resolved` or `netconfig` from overwriting the DNS configuration on container restart. This ensures containers always have working internet access.

## Shared Folders

MELISA can mount host directories into containers using `lxc.mount.entry` directives in the container's config file. This creates a bind mount, giving the container read-write access to a host path:

```
melisa --share mybox /home/user/code /workspace
```

Internally, MELISA:
1. Resolves the host path to an absolute path
2. Appends the mount entry to `/var/lib/lxc/mybox/config`
3. Fixes ownership to `100000:100000` (the unprivileged UID mapping) so the container user has access

To remove a mount:

```
melisa --reshare mybox /home/user/code /workspace
```

This atomically rewrites the config file, removing only the specific `lxc.mount.entry` line and its associated comment tag.

> **Important:** Shared folder changes require a container restart to take effect.

## Container States

| State | Description |
|-------|-------------|
| `STOPPED` | Container exists but is not running. Filesystem is preserved. |
| `RUNNING` | Container is active. Processes are executing, networking is up. |
| `FROZEN` | Container processes are paused (not currently exposed in CLI). |

## Pre-flight Verification

Before creating a container, MELISA verifies the host runtime environment by checking `/sys/class/net/lxcbr0`. If the bridge is missing, it attempts automatic repair by restarting `lxc-net.service`. If repair fails, creation is aborted with an actionable error message.