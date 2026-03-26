# Container Interaction Commands

## `--use <n>`

**Access:** All users

Attaches an interactive TTY shell to a running container. This is the primary way to "enter" a container and work inside it.

```
melisa@host:~> melisa --use mybox
root@mybox:/#
```

Executes `lxc-attach -P /var/lib/lxc -n mybox`. The container must be in `RUNNING` state — start it first with `--run <n>` if needed.

**What happens when you `--use`:**
- Your terminal is attached to the container's PID namespace
- You get a shell inside the container (typically bash or sh)
- `Ctrl+D` or `exit` returns you to the MELISA prompt
- The container keeps running after you detach

---

## `--send <n> <command>`

**Access:** All users

Executes a command inside a container **non-interactively**. Output is streamed back to your terminal. The container must be running.

```
melisa@host:~> melisa --send mybox apt list --installed
melisa@host:~> melisa --send mybox python3 -c "import sys; print(sys.version)"
melisa@host:~> melisa --send mybox bash -c "ls /opt && echo done"
```

Executes `lxc-attach -P /var/lib/lxc -n mybox -- <command> <args...>`.

**Multi-word commands:** Pass the entire command as sequential arguments:

```
melisa@host:~> melisa --send mybox bash -c "apt-get install -y curl"
```

---

## `--info <n>`

**Access:** All users

Reads and displays the MELISA metadata file (`/etc/melisa-info`) from inside a container's rootfs. Does **not** require the container to be running — reads directly from the host filesystem.

```
melisa@host:~> melisa --info mybox

Searching metadata for container: mybox...

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

**Error cases:**
- Container not found: standard filesystem error
- Container exists but wasn't created by MELISA: `[ERROR] Container 'mybox' lacks MELISA metadata. It may not have been provisioned via the MELISA Engine.`

The metadata path is: `/var/lib/lxc/<n>/rootfs/etc/melisa-info`

---

## `--upload <n> <dest_path>`

**Access:** All users

Uploads a compressed tarball (piped from stdin) and extracts it inside the container at the specified destination path.

This command is designed to receive a **piped tarball stream** — it is primarily used by the MELISA client's `exec_upload` function, but can also be used directly:

```bash
# From the host terminal:
tar -czf - -C /home/user/myproject . | \
  melisa --upload mybox /workspace
```

**Internal behavior:**
```bash
lxc-attach -P /var/lib/lxc -n mybox -- bash -c \
  "mkdir -p /workspace && tar -xzf - -C /workspace"
```

The command inherits stdin from the calling process, accepts the tarball stream, creates the destination directory if needed, and extracts in place.

---

## `--share <n> <host_path> <container_path>`

**Access:** Administrator only

Mounts a host directory into a running (or stopped) container by adding an `lxc.mount.entry` directive to the container's config file.

```
melisa@host:~> melisa --share mybox /home/user/code /workspace
```

**What happens internally:**

1. The host path is canonicalized to an absolute path
2. If the container's config exists, the mount entry is appended:
   ```
   # Shared Folder mapped by MELISA
   lxc.mount.entry = /home/user/code workspace none bind,create=dir 0 0
   ```
3. Ownership is fixed to `100000:100000` (the unprivileged LXC UID mapping) so the container user has proper access

> **Restart required:** The container must be restarted for the mount to take effect.

**Unprivileged UID mapping:** Because MELISA uses LXC user namespace mapping (UID `100000–165535`), any shared folder on the host must be owned by UID `100000` for the container to access it. MELISA applies this automatically.

---

## `--reshare <n> <host_path> <container_path>`

**Access:** Administrator only

Unmounts a previously shared host directory by removing its `lxc.mount.entry` from the container config.

```
melisa@host:~> melisa --reshare mybox /home/user/code /workspace
```

**What happens internally:**

1. Canonicalizes the host path to match the exact string in the config file
2. Reads the config file line by line
3. Removes the specific `lxc.mount.entry` line and its preceding `# Shared Folder mapped by MELISA` comment tag
4. Atomically rewrites the config file

If the entry is not found: `[SKIP] Shared folder mapping was not found in the configuration.`

> **Restart required:** The container must be restarted for the change to take effect.

---

## Usage Pattern: Remote Script Execution

A common pattern enabled by `--send` and `--upload` together is deploying and running code inside a container from the client side:

```bash
# From the MELISA client (workstation):

# 1. Upload a script
melisa upload mybox ./my_scripts /tmp/

# 2. Execute it
melisa run mybox /tmp/setup.sh

# Or interactively with TTY (for scripts that need user input):
melisa run-tty mybox /tmp/interactive_installer.sh
```

The client handles the SSH tunneling and streaming; the server's `--upload` and `--send` do the actual work inside the container.