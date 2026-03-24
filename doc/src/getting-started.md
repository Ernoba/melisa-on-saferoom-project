# Getting Started

## 🛠️ Installing the Server (Host)

**MELISA (Management Environment Linux Sandbox)** is designed to run on a Linux host—the primary environment where all isolated containers reside. Currently, the automated setup is optimized exclusively for **Fedora Linux** and related distributions that utilize the `dnf` package manager. 

> [!NOTE]
> Support for other host distributions, such as Ubuntu/Debian or Arch, is planned for future releases.

### What you’ll need

Before we begin the ritual, ensure you have the following components ready:

* **Fedora Linux:** Either Workstation or Server edition.
* **Physical/Console Access:** For security reasons, the setup command **refuses to run over SSH**. You must be physically at the terminal to prevent remote attackers from initializing your system.
* **Internet Connection:** Needed to fetch the necessary packages and container templates.
* **Sudo Privileges:** You must have root access to manage LXC configurations and networking.

### Installation Steps

Make sure you have **Rust** installed (use [rustup](https://rustup.rs/) if you don’t). Then, follow these steps:

 1. **Compile from Source**
    Clone the repository and compile the MELISA binary using the Rust compiler:

    ```bash
    git clone [https://github.com/ernoba/melisa-on-saferoom-project.git](https://github.com/ernoba/melisa-on-saferoom-project.git)
    cd melisa-on-saferoom-project
    cargo build
    ```

2.  **Initialize the System**
    Run MELISA program:
    
    ```bash
    sudo -E ./target/debug/melisa
    ```

    **Verify MELISA running:**
    You can now verify that MELISA is running correctly by executing the binary:

    ```text
    ███╗   ███╗███████╗██║     ██║███████╗███████╗ 
    ████╗ ████║██╔════╝██║     ██║██╔════╝██╔══██╗ 
    ██╔████╔██║█████╗  ██║     ██║███████╗███████║ 
    ██║╚██╔╝██║██╔══╝  ██║     ██║╚════██║██╔══██║ 
    ██║ ╚═╝ ██║███████╗███████╗██║███████║██║  ██║ 
    ╚═╝     ╚═╝╚══════╝╚══════╝╚═╝╚══════╝╚═╝  ╚═╝ 
        [ MANAGEMENT ENVIRONMENT LINUX SANDBOX ]  

    ┌─── SYSTEM TELEMETRY & STATUS ──────────────────────────────────────┐
    │ TIMESTAMP  :: 2026-03-20 16:25:23
    │ KERNEL_ID  :: FEDORA LINUX
    │ HOST_NODE  :: FEDORA
    │ PROCESSOR  :: AMD Ryzen 7 7435HS
    │ GPU_STATUS :: NVIDIA Corporation GA107  (rev a1)
    │ RAM_USAGE  :: 5878MB / 15794MB (37%)
    │ ------------------------------------------------------------------
    │ PROTOCOL   :: SECURE ISOLATION ACTIVE
    │ DIRECTIVE  :: MAXIMUM PERFORMANCE // ZERO INEFFICIENCY
    └────────────────────────────────────────────────────────────────────┘

    >>> ALL SYSTEMS OPERATIONAL. SECURE SESSION GRANTED.
    ENTER COMMAND: Session ID: saferoom | Environment: SECURE_JAIL
    Authenticated as melisa. Access granted.
    melisa@saferoom:../saferoom> 
    ```

    **Run the - -setup**
    routine to install system dependencies (`LXC`, `bridge-utils`, `openssh-server`) and configure the network bridge:
    ```bash
    melisa@saferoom:../saferoom> melisa --setup
    ```

    ### What the Setup Process Does

    During the installation, the script will perform the following actions:

    * **System Update:** Updates package repositories using `dnf update -y`.
    * **Dependency Installation:** Installs `LXC`, `lxc-templates`, `libvirt`, `bridge-utils`, `openssh-server`, and `firewalld`.
    * **Kernel Configuration:** Loads the `veth` kernel module.
    * **Service Management:** Enables and starts `lxc.service`, `lxc-net.service`, `sshd`, and `firewalld`.
    * **Binary Deployment:** Deploys the MELISA binary to `/usr/local/bin` with the **SUID bit** enabled (allowing standard users to run it).
    * **Project Workspace:** Creates the master projects directory at `/opt/melisa/projects` with proper permissions.
    * **Network Security:** Configures the firewall to open SSH ports and sets the `lxcbr0` interface as trusted.
    * **User Mapping:** Automatically sets up `subuid` and `subgid` mappings for the user executing the setup.
    * **System Hardening:** Secures `/home` permissions (`chmod 711`) to prevent users from listing other users' directories.
    * **Shell Registration:** Registers `/usr/local/bin/melisa` as a valid shell in `/etc/shells`.
    * **Privilege Management:** Deploys a global `sudoers` rule allowing all users to execute `melisa` without a password.

    > [!WARNING]
    > **Direct Host Access Required** > The setup command must be run directly on the host console — **not over SSH**. This is a deliberate design choice to prevent remote attackers from initialising the system. After installation, you can manage everything remotely using the client, but the initial bootstrap is a one‑time physical operation.

    Upon completion, a summary will be displayed, and the MELISA boot banner will appear the next time you launch the program.

3.  **Verification**
    Once the setup completes, you can check that everything is working by running:
    ```bash
    melisa --list
    ```
    If you see an empty list (no containers yet), that’s normal. The important thing is that the command runs without errors.
    Your host is now ready to host containers. Congratulations! 🎉

> [!IMPORTANT]
> The installer will automatically verify the LXC configuration and deploy the binary to `/usr/local/bin/melisa` for global access.
>
> The current installer relies strictly on `dnf` commands. Attempting to run the setup on non-Fedora hosts will result in a terminal failure to prevent system inconsistency.

## 🖥️ Installing the Client (Remote Workstation)

Now that the server is up and running, you probably want to control it from your laptop. MELISA comes with a lightweight client written in Bash that talks to the server over SSH.

### Step 1: Run the client installer
On your local machine (laptop, desktop, etc.), go to the `melisa_client` directory inside the repository:

```bash
cd melisa/src/melisa_client
./install.sh
```

#### The installer will:

* **Create ~/.local/bin** (if it doesn’t exist) and copy the melisa script there.
* **Copy the helper modules** (auth.sh, exec.sh, utils.sh) into ~/.local/share/melisa/.
* **Make sure ~/.local/bin** is in your PATH (by appending to your ~/.bashrc if necessary).

> [!IMPORTANT]
> After installation, you can run melisa from any terminal. Type ```melisa``` to see the help message.

### Step 2: Add your server profile

Before you can do anything, you need to tell the client how to reach your server. The client uses SSH under the hood, so you’ll need SSH access to the host (and the host must have the SSH server running — the **MELISA** setup already enabled it).

#### Add a profile with:
```bash
melisa auth add myserver root@192.168.1.100
```

> [!WARNING]
> Replace myserver with a nickname you like, and root@192.168.1.100 with the actual IP address of your server and the username you want to connect with (usually root if you set up the host with sudo access).

**The command will:**

* Check if you have an SSH key in `~/.ssh/id_rsa`. If not, it generates one automatically.
* Run `ssh-copy-id` to copy your public key to the server (you’ll be prompted for the server’s password once).
* Configure **SSH multiplexing** (`ControlMaster`) to keep connections alive and speed up subsequent commands.
* Store the profile in `~/.config/melisa/profiles.conf`.
* Set the newly added profile as active.

**Manage your profiles:**

* **List all profiles:**
  ```bash
  melisa auth list
  ```

* **Switch between them:**
  ```bash
  melisa auth switch anotherserver
  ```

### Step 3: Test the connection

Now try a simple command on the server, for example listing containers:

```bash
melisa --list
```

If everything is configured correctly, you should see the output from the server (probably an empty list). Behind the scenes, the client forwards the `--list` flag over SSH to the server’s **MELISA** binary.