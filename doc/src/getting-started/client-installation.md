# Client Installation

The MELISA client is a modular Bash script that runs on your local workstation (laptop, desktop, CI runner — any machine with `ssh` and `rsync`). It communicates with the MELISA host exclusively over SSH, so it works from anywhere in the world.

## Prerequisites

| Tool | Check | Install |
|------|-------|---------|
| `ssh` (OpenSSH client) | `ssh -V` | `dnf install openssh-clients` / `apt install openssh-client` |
| `rsync` | `rsync --version` | `dnf install rsync` / `apt install rsync` |
| `git` | `git --version` | `dnf install git` / `apt install git` |

> The client installer will abort with a clear `[FATAL ERROR]` message if `ssh` is missing.

---

## Step 1: Run the Installer

Navigate to the `melisa_client` directory inside the cloned repository and execute the installer:

```bash
cd melisa-on-saferoom-project/src/melisa_client
./install.sh
```

### What the Installer Does

The installer is designed to be **idempotent** (safe to run multiple times) and **non-destructive**:

1. **Provisions local directories:**
   - `~/.local/bin/` — executable location
   - `~/.local/share/melisa/` — module library location
   - `~/.config/melisa/` — profile and registry storage

2. **Sanitizes ownership** — runs `sudo chown -R $USER` on the target directories to reclaim any files accidentally created by a previous root-owned installation.

3. **Deploys core files:**
   - `~/.local/bin/melisa` — the main entry-point script
   - `~/.local/share/melisa/auth.sh` — authentication & profile management module
   - `~/.local/share/melisa/exec.sh` — remote execution & project sync engine
   - `~/.local/share/melisa/utils.sh` — shared utilities, logging, and SSH key management
   - `~/.local/share/melisa/db.sh` — local project path registry

4. **Sets execution permissions** (`chmod +x`) on all deployed scripts.

5. **PATH Registration** — detects your active shell and appends `~/.local/bin` to your `$PATH` if it isn't already there:
   - Bash → `~/.bashrc`
   - Zsh → `~/.zshrc`
   - Other → `~/.profile`

After installation, if your PATH was modified, apply it immediately:

```bash
source ~/.bashrc   # or ~/.zshrc
```

### Verify Installation

```bash
melisa
```

Expected output (the help screen):

```
MELISA REMOTE MANAGER - CLI CLIENT
Usage: melisa <command> [arguments]

AUTHENTICATION & CONNECTIONS:
  auth add <n> <user@ip>  : Register a new remote MELISA server
  auth switch <n>         : Switch active session to another server
  auth list               : Display all registered remote servers
  auth remove <n>         : Unregister and delete a remote server

PROJECT SYNCHRONIZATION:
  clone <n> [--force]     : Clone a project workspace from the host
  sync                    : Push local workspace modifications to the host
  get <n> [--force]       : Pull the latest master data into local workspace

REMOTE OPERATIONS (Executed on Active Server):
  run <container> <file>  : Execute a local script inside a remote container
  run-tty <cont> <file>   : Execute a script interactively (Foreground/TTY)
  upload <cont> <dir> <dst>: Transfer a local directory into a container
  shell                   : Open a direct interactive SSH shell to the host
  --list / --active       : Enumerate provisioned containers on the active server
```

---

## Step 2: Register Your Server Profile

Before issuing any command, you must tell the client how to reach your MELISA host. The client stores named profiles so you can manage multiple servers.

```bash
melisa auth add myserver root@192.168.1.100
```

Replace `myserver` with any nickname and `root@192.168.1.100` with your actual server address.

### What `auth add` Does Internally

1. **SSH Key Verification** — Checks for `~/.ssh/id_ed25519` or `~/.ssh/id_rsa`. If neither exists, generates a modern **Ed25519 keypair** automatically:
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N "" -q
   ```

2. **Key Distribution** — Runs `ssh-copy-id` to install your public key on the server (you'll be prompted for the server password **once only**).

3. **SSH Multiplexing** — Configures `ControlMaster auto` and `ControlPath` in `~/.ssh/config` for the specific host. This keeps a persistent SSH connection alive so subsequent commands are nearly instantaneous.

4. **Profile Storage** — Saves the mapping `myserver=root@192.168.1.100` to `~/.config/melisa/profiles.conf`.

5. **Activation** — Sets the newly added profile as the currently active server.

### Profile Files

| File | Purpose |
|------|---------|
| `~/.config/melisa/profiles.conf` | Named server registry (`name=user@host` format) |
| `~/.config/melisa/active` | Contains the name of the currently active server |
| `~/.config/melisa/registry` | Local project path database (`name|/absolute/path` format) |

---

## Step 3: Test the Connection

```bash
melisa --list
```

If this returns the container list from your server (empty or populated), your client is configured correctly. Behind the scenes, the client:

1. Reads the active server from `~/.config/melisa/active`
2. Resolves the connection string from `~/.config/melisa/profiles.conf`
3. Executes `ssh -t root@192.168.1.100 "melisa --list"` over the multiplexed connection

---

## Managing Multiple Servers

MELISA's profile system is designed for multi-server environments:

```bash
# Register a second server
melisa auth add production root@10.0.0.5

# List all registered servers
melisa auth list
# Output:
# === MELISA REMOTE SERVER REGISTRY ===
#   * myserver    (root@192.168.1.100)  <- [ACTIVE]
#     production  (root@10.0.0.5)

# Switch active server
melisa auth switch production

# Remove a server
melisa auth remove myserver
```

All subsequent commands automatically target the active server.