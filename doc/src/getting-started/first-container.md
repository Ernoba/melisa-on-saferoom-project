# Your First Container

With the server running and the client connected, you're ready to provision your first isolated environment. This page walks through the complete container lifecycle from creation to deletion.

---

## 1. Search for a Distribution

MELISA pulls container images from the official LXC image servers. To see what's available, use `--search`:

```
melisa@host:~> melisa --search ubuntu
```

This queries the LXC distribution registry and filters results by keyword. The output looks like:

```
[INFO] Synchronizing distribution list...

  CODE                          DISTRIBUTION     RELEASE   ARCH
  ubuntu-focal-amd64            ubuntu           focal     amd64
  ubuntu-jammy-amd64            ubuntu           jammy     amd64
  ubuntu-noble-amd64            ubuntu           noble     amd64
  ...
```

The **CODE** column is what you'll use with `--create`. It's a unique slug combining the distro name, release, and architecture.

To see everything available:

```
melisa@host:~> melisa --search
```

> **Note:** The first call fetches the list from the LXC remote and caches it locally. Subsequent calls use the cache for speed.

---

## 2. Create the Container

Use the code from the search results to provision a new container:

```
melisa@host:~> melisa --create mybox ubuntu-jammy-amd64
```

You'll see a live progress spinner as MELISA:

1. **Pre-flight checks** — Verifies `lxcbr0` bridge is active; attempts auto-repair if not.
2. **Pulls the rootfs** — Downloads and extracts the Ubuntu Jammy base image via `lxc-create -t download`.
3. **Injects metadata** — Writes `/etc/melisa-info` inside the container with the instance UUID, distro info, and creation timestamp.
4. **Configures networking** — Injects the `lxcbr0` network configuration into the container's config file.
5. **Locks DNS** — Writes `/etc/resolv.conf` inside the container and sets `chattr +i` to prevent `systemd-resolved` from overwriting it.
6. **Starts the container** — Runs `lxc-start` and polls for network readiness (DHCP assignment).
7. **Initial package update** — Once the container has network access, runs the appropriate package manager update (`apt update`, `dnf update`, etc.) automatically based on the detected distro.

Expected output:

```
[INFO] Provisioning container 'mybox' ...
--- Creating Container: mybox (ubuntu-jammy-amd64) ---
[SUCCESS] Container successfully created.
[INFO] Starting container for initial setup...
[INFO] Waiting for network... (attempt 1/20)
[INFO] Network ready. Running initial package setup...
[SUCCESS] Container successfully provisioned!
```

---

## 3. Inspect the Container

After creation, verify the container's metadata:

```
melisa@host:~> melisa --info mybox
```

Output:

```
--- [ MELISA CONTAINER INFO ] ---
MELISA_INSTANCE_NAME=mybox
MELISA_INSTANCE_ID=f47ac10b-58cc-4372-a567-0e02b2c3d479
DISTRO_SLUG=ubuntu-jammy-amd64
DISTRO_NAME=ubuntu
DISTRO_RELEASE=jammy
ARCHITECTURE=amd64
CREATED_AT=2026-03-20T16:30:00+07:00
----------------------------------
```

---

## 4. List All Containers

```
melisa@host:~> melisa --list
```

To see only running containers:

```
melisa@host:~> melisa --active
```

---

## 5. Start and Enter the Container

Start the container if it isn't already running:

```
melisa@host:~> melisa --run mybox
```

Then attach to it with an interactive shell:

```
melisa@host:~> melisa --use mybox
root@mybox:/#
```

You are now inside the container. The host filesystem is untouched. Do whatever you want in here — install packages, break things, compile code — it's all isolated.

Exit with `Ctrl+D` or `exit` to return to the MELISA prompt.

---

## 6. Send a Command Without Entering

For scripted workflows, you can execute a single command inside the container without entering it interactively:

```
melisa@host:~> melisa --send mybox apt list --installed
```

All output is streamed back to your terminal.

---

## 7. Stop the Container

When you're done with a session:

```
melisa@host:~> melisa --stop mybox
```

---

## 8. Delete the Container

When you want to permanently destroy the container and free disk space:

```
melisa@host:~> melisa --delete mybox
```

MELISA will prompt for confirmation:

```
Are you sure you want to permanently delete 'mybox'? (y/N):
```

Press `y` to confirm. The container's rootfs, configuration, and all data inside it are deleted. **This is irreversible.**

---

## Container Lifecycle Summary

```
--search   →  Find a distribution code
--create   →  Provision & auto-configure the container
--run      →  Start the container
--use      →  Attach interactive shell
--send     →  Execute a single command
--stop     →  Gracefully shut down
--delete   →  Permanently destroy (confirmation required)
```