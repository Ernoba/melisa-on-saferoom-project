# Users & Roles

MELISA implements its own layer of identity and access management on top of Linux system users. Every MELISA user is a real Linux system user but with a **customized sudoers policy** and the **MELISA jail shell** as their login shell.

---

## How MELISA Users Work

When a user SSHes into the MELISA host, the following happens:

```
SSH Connection
     │
     ▼
PAM / sshd authentication
     │
     ▼
Login shell lookup in /etc/passwd
     │
     ▼
/usr/local/bin/melisa   ← The MELISA binary IS the shell
     │
     ▼
MELISA interactive prompt
melisa@hostname:~>
```

The user never gets a bash shell. They land directly inside the MELISA environment. The only commands available to them are MELISA commands — plus any commands that the MELISA executor passes through to the underlying bash (the fallback handler).

---

## The Two Roles

### Administrator

Admins have **full management privileges**. They can:
- Create and delete other users
- Create and delete projects
- Invite/remove users from projects
- Force-pull code from any user's workspace
- Delete containers
- Share/unshare host directories with containers
- Run the `--setup` routine
- Clear command history
- Search and create containers from any distribution

**Sudoers policy for admins** (deployed to `/etc/sudoers.d/melisa_<username>`):

```
username ALL=(ALL) NOPASSWD: \
  /usr/bin/lxc-*, /bin/lxc-*,
  /usr/sbin/lxc-*, /sbin/lxc-*,
  /usr/share/lxc/templates/lxc-download *,
  /usr/bin/git *, /bin/git *,
  /usr/local/bin/melisa *,
  /usr/bin/mkdir -p *, /bin/mkdir -p *,
  /usr/bin/rm -f *, /bin/rm -f *,
  /usr/bin/bash -c *, /bin/bash -c *,
  /usr/bin/tee *, /bin/tee *,
  /usr/bin/chattr *, /bin/chattr *,
  /usr/sbin/useradd *, /sbin/useradd *,
  /usr/sbin/userdel *, /sbin/userdel *,
  /usr/bin/passwd *, /bin/passwd *,
  /usr/bin/pkill *, /bin/pkill *,
  /usr/bin/chmod *, /bin/chmod *,
  /usr/bin/chown *, /bin/chown *,
  ...
```

### Standard User

Standard users can manage **their own containers and projects** they've been invited to. They cannot manage other users or create new projects.

**Sudoers policy for standard users** (subset of above):

```
username ALL=(ALL) NOPASSWD: \
  /usr/bin/lxc-*, /bin/lxc-*,
  /usr/sbin/lxc-*, /sbin/lxc-*,
  /usr/share/lxc/templates/lxc-download *,
  /usr/bin/git *, /bin/git *,
  /usr/local/bin/melisa *,
  /usr/bin/mkdir -p *, /bin/mkdir -p *,
  /usr/bin/rm -f *, /bin/rm -f *,
  /usr/bin/bash -c *, /bin/bash -c *,
  /usr/bin/tee *, /bin/tee *,
  /usr/bin/chattr *, /bin/chattr *
```

---

## Capability Matrix

| Capability | Standard User | Administrator |
|-----------|:-------------:|:-------------:|
| Create container | ✅ | ✅ |
| Delete container | ❌ | ✅ |
| Start/stop container | ✅ | ✅ |
| Enter container | ✅ | ✅ |
| Send command to container | ✅ | ✅ |
| Upload files to container | ✅ | ✅ |
| Share host folder | ❌ | ✅ |
| List containers | ✅ | ✅ |
| View container info | ✅ | ✅ |
| Add user | ❌ | ✅ |
| Delete user | ❌ | ✅ |
| Change own password | ✅ | ✅ |
| Change any password | ❌ | ✅ |
| Upgrade user role | ❌ | ✅ |
| Create project | ❌ | ✅ |
| Delete project | ❌ | ✅ |
| Invite to project | ❌ | ✅ |
| Remove from project | ❌ | ✅ |
| List projects | ✅ | ✅ |
| Sync own project | ✅ | ✅ |
| Force-pull user's code | ❌ | ✅ |
| Run `--setup` | ❌ | ✅ |
| Clear history | ❌ | ✅ |

---

## User Home Directory Isolation

When a MELISA user is created, their home directory is set to mode `700`:

```bash
chmod 700 /home/<username>
```

Combined with `chmod 711 /home` (set by `--setup`), this creates a two-layer privacy model:
- `711` on `/home`: Other users can traverse into the directory (needed for `cd /home/alice`) but cannot list its contents (no `ls /home`)
- `700` on `/home/alice`: Nobody except Alice (and root) can read her files

---

## User Lifecycle

```bash
# Create a new user (interactive: prompts for role + password)
melisa --add alice

# Update a user's password
melisa --passwd alice

# Promote to administrator
melisa --upgrade alice

# List all registered users
melisa --user

# Delete a user (kills processes, removes home, cleans sudoers)
melisa --remove alice

# Clean up stale sudoers files (after manual deletions)
melisa --clean
```

### What `--remove` Does

The deletion process is designed to be clean and complete:

1. Sends `SIGKILL` to all processes owned by the user (`pkill -u <username>`) to prevent "device busy" errors
2. Deletes the system user and home directory (`userdel -r -f <username>`)
3. Removes the user's custom sudoers file (`/etc/sudoers.d/melisa_<username>`)

### What `--clean` Does

The `--clean` command scans `/etc/sudoers.d/` for files matching the `melisa_*` pattern and removes any whose corresponding system user no longer exists. This handles cases where users were deleted manually outside MELISA.