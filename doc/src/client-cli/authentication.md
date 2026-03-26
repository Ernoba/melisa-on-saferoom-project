# Authentication & Profiles

The `auth` subcommand manages the registry of remote MELISA servers. All profile data is stored locally in `~/.config/melisa/`.

---

## `melisa auth add <name> <user@ip>`

Registers a new remote MELISA server under a memorable nickname.

```bash
melisa auth add myserver root@192.168.1.100
melisa auth add homelab alice@10.0.0.5
melisa auth add production deploy@prod.example.com
```

### What Happens Internally

**1. Directory Initialization (`init_auth`)**

Before any auth operation, `init_auth` ensures the configuration directory exists with proper permissions:

```bash
mkdir -p ~/.config/melisa
chmod 700 ~/.config/melisa
touch ~/.config/melisa/profiles.conf
touch ~/.config/melisa/active
```

**2. SSH Key Check (`ensure_ssh_key`)**

Checks for `~/.ssh/id_ed25519` or `~/.ssh/id_rsa`. If neither exists:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N "" -q
```

Generates a passphrase-less Ed25519 keypair (required for automated CLI operations without password prompts).

**3. Public Key Distribution**

```bash
ssh-copy-id -i ~/.ssh/id_ed25519.pub root@192.168.1.100
```

You'll be prompted for the **server's password once**. After this, all subsequent connections use key authentication.

**4. SSH Multiplexing Configuration**

Appends to `~/.ssh/config`:

```
Host 192.168.1.100
  ControlMaster auto
  ControlPath ~/.ssh/melisa_mux_%h_%p_%r
  ControlPersist 10m
```

`ControlMaster auto` keeps a single master SSH connection alive for 10 minutes. All subsequent commands reuse this connection, making commands nearly instantaneous instead of incurring SSH handshake overhead on every call.

**5. Profile Storage**

Appends to `~/.config/melisa/profiles.conf`:

```
myserver=root@192.168.1.100
```

**6. Auto-activation**

Writes `myserver` to `~/.config/melisa/active`, making it the default for all future commands.

---

## `melisa auth switch <name>`

Changes the active server without re-configuring anything:

```bash
melisa auth switch production
```

```
[SUCCESS] Successfully switched active connection to server: production
```

Validates that the profile name exists in `profiles.conf` before writing to `active`. If not found:

```
[ERROR] Server profile 'typo' not found! Execute 'melisa auth list' to view available profiles.
```

---

## `melisa auth list`

Displays all registered servers with a clear active marker:

```bash
melisa auth list
```

```
=== MELISA REMOTE SERVER REGISTRY ===
  * myserver      (root@192.168.1.100)  <- [ACTIVE]
    production    (deploy@prod.example.com)
    homelab       (alice@10.0.0.5)
```

The `*` prefix and `<- [ACTIVE]` suffix are shown on the currently active server. Reads directly from `profiles.conf`.

---

## `melisa auth remove <name>`

Unregisters a server profile:

```bash
melisa auth remove homelab
```

Removes the `homelab=...` line from `profiles.conf`. Does **not** remove the SSH key or the multiplexing socket — only the profile registration.

If you remove the currently active server, you'll need to run `auth switch` to set a new active server before issuing any commands.

---

## Profile Files Reference

### `~/.config/melisa/profiles.conf`

Plain key-value store, one profile per line:

```
myserver=root@192.168.1.100
production=deploy@prod.example.com
homelab=alice@10.0.0.5
```

### `~/.config/melisa/active`

Single line containing the active profile name:

```
myserver
```

### `~/.config/melisa/registry`

Pipe-delimited project path mappings:

```
myapp|/home/user/projects/myapp
backend|/home/user/work/backend
```

This registry is managed automatically by `clone` and `get` commands. `sync` reads it to locate your project root.

---

## Resolving the Active Connection

The `get_active_conn` function (used internally by all `exec.sh` functions) resolves the current connection string:

```bash
get_active_conn() {
    local name=$(cat "$ACTIVE_FILE" 2>/dev/null)
    if [ -z "$name" ]; then echo ""; return; fi
    grep "^${name}=" "$PROFILE_FILE" | cut -d'=' -f2
}
```

Returns empty string if no active profile is set, which causes `ensure_connected` to abort with an error and the tip to add a server.