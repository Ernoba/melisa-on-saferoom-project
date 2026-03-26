# Container Lifecycle Commands

## `--search [keyword]`

**Access:** Administrator only

Queries the LXC distribution registry for available container images. On first call, fetches from the LXC remote and caches locally. Subsequent calls use the cache for instant results.

```
melisa@host:~> melisa --search ubuntu
```

```
melisa@host:~> melisa --search debian
```

```
melisa@host:~> melisa --search           # Show all available distributions
```

**Output format:**

```
[INFO] Synchronizing distribution list...

  CODE                          DISTRIBUTION     RELEASE    ARCH
  ubuntu-focal-amd64            ubuntu           focal      amd64
  ubuntu-jammy-amd64            ubuntu           jammy      amd64
  ubuntu-noble-amd64            ubuntu           noble      amd64
  debian-bookworm-amd64         debian           bookworm   amd64
  alpine-3.19-amd64             alpine           3.19       amd64
  fedora-39-amd64               fedora           39         amd64
  ...
```

The **CODE** column is the slug you pass to `--create`. It uniquely identifies a distro/release/arch combination.

**Cache behavior:** The distribution list is fetched once and cached. If it's cached, you'll see:
```
[INFO] Validating distribution code 'ubuntu-jammy-amd64' against local cache.
```

---

## `--create <name> <code>`

**Access:** Administrator only

Provisions a new LXC container from a distribution code obtained via `--search`.

```
melisa@host:~> melisa --create mybox ubuntu-jammy-amd64
```

### Creation Pipeline (Detailed)

The creation process runs through these stages automatically:

**Stage 0 — Pre-flight Check**
Verifies `/sys/class/net/lxcbr0` exists. If the bridge is missing, attempts `systemctl restart lxc-net.service`. Aborts if the bridge cannot be brought up.

**Stage 1 — rootfs Download**
```bash
sudo lxc-create -P /var/lib/lxc -t download -n mybox \
  -- -d ubuntu -r jammy -a amd64
```
Downloads and extracts the base container image. May take several minutes on first download depending on connection speed.

**Stage 2 — Metadata Injection**
Writes `/var/lib/lxc/mybox/rootfs/etc/melisa-info` atomically:
```
MELISA_INSTANCE_NAME=mybox
MELISA_INSTANCE_ID=<uuid-v4>
DISTRO_SLUG=ubuntu-jammy-amd64
DISTRO_NAME=ubuntu
DISTRO_RELEASE=jammy
ARCHITECTURE=amd64
CREATED_AT=<rfc3339-timestamp>
```

**Stage 3 — Network Configuration**
Appends network settings to `/var/lib/lxc/mybox/config`:
```
lxc.net.0.type = veth
lxc.net.0.link = lxcbr0
lxc.net.0.flags = up
```

**Stage 4 — DNS Lock**
Writes `/etc/resolv.conf` inside the container with working DNS servers and sets `chattr +i` to make it immutable against `systemd-resolved` overwrites.

**Stage 5 — Initial Start & Wait**
Starts the container and polls for network readiness every 2 seconds, up to 20 attempts (40 second timeout). Checks for a DHCP-assigned IP on the container's network interface.

**Stage 6 — Package Manager Initialization**
Once the container has network, runs the appropriate update command based on the detected distribution:

| Distro | Command |
|--------|---------|
| Ubuntu / Debian | `apt-get update -y` |
| Fedora / RHEL | `dnf update -y` |
| Alpine | `apk update` |
| Arch | `pacman -Sy` |

### Error Handling

| Error | Response |
|-------|----------|
| Container already exists | `[WARNING] Skipping creation.` (non-fatal) |
| GPG signature failure | Suggests `gpg --recv-keys` fix |
| Download failure | Requests internet connectivity check |
| Network timeout | Skips package update, reports timeout |

---

## `--list`

**Access:** All users

Enumerates all LXC containers stored in `/var/lib/lxc/`:

```
melisa@host:~> melisa --list
```

```
[INFO] Retrieving container inventory...

NAME      STATE    AUTOSTART   GROUPS   IPV4          IPV6   UNPRIVILEGED
mybox     RUNNING  0           -        10.0.3.102    -      false
devlab    STOPPED  0           -        -             -      false
```

Internally executes `lxc-ls -P /var/lib/lxc --fancy`.

---

## `--active`

**Access:** All users

Same as `--list` but filters to only show **running** containers:

```
melisa@host:~> melisa --active
```

Internally executes `lxc-ls -P /var/lib/lxc --fancy --active`.

---

## `--run <name>`

**Access:** All users

Starts a stopped container:

```
melisa@host:~> melisa --run mybox
```

Executes `lxc-start -P /var/lib/lxc -n mybox`. Returns immediately; the container starts in the background. Use `--active` to verify it's running.

---

## `--stop <name>`

**Access:** All users

Gracefully shuts down a running container:

```
melisa@host:~> melisa --stop mybox
```

Executes `lxc-stop -P /var/lib/lxc -n mybox`. Sends SIGPWR to init process inside the container, allowing graceful shutdown.

---

## `--delete <name>`

**Access:** Administrator only

Permanently destroys a container and its entire filesystem. Requires interactive confirmation:

```
melisa@host:~> melisa --delete mybox
[INFO] Validating deletion request for 'mybox'...
Are you sure you want to permanently delete 'mybox'? (y/N): y
```

Press `Enter` alone (empty input) defaults to **No** and aborts.

The deletion reads input asynchronously via `tokio::io::BufReader` to avoid blocking the async runtime.

> **This operation is irreversible. All data inside the container is permanently lost.**