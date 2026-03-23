# Getting Started

### Laying the Foundation of the Manor
Before building your pristine, isolated sanctuaries, we must lay the foundation. **MELISA** (Management Environment Linux Sandbox) is a high-performance manager written in **Rust** designed to end "Host Pollution" and system chaos. By setting up the **MELISA Host**, you grant the engine authority to carve out isolated spaces within the Linux kernel, ensuring every workspace is a "Clean Room" where variables never leak.

---

### The Philosophy of the "Physical Handshake"
MELISA demands your physical presence for the initial setup—a ritual we call the **Physical Handshake**.

* **Host Mode Only:** The `setup` routine is strictly **forbidden via SSH**; only a physical terminal session can initialize the system.
* **Absolute Authority:** You must trigger this process with **root/sudo privileges** to grant the engine its necessary power.
* **Security by Presence:** This prevents remote attackers from influencing core initialization before your defenses are even built.

---

### What to Expect
We will guide your transition from a standard installation to a powerful orchestration node through these steps:

1.  **The Host Ritual:** You will build the command center from source, leveraging the speed and safety of the **Rust** language.
2.  **The Great Initialization:** Triggering the `--setup` routine acts as a master architect:
    * **Deploying Tools:** Installs **LXC**, `libvirt`, and `bridge-utils` while configuring network bridges.
    * **Binary Ascension:** Moves the binary to `/usr/local/bin/melisa` and applies **SUID bits (4755)** for controlled privilege escalation.
    * **System Hardening:** Secures the manor by making `/home` unlistable and establishing the **Master Projects** directory at `/opt/melisa/projects`.
3.  **Remote Ascension:** Once the foundation is set, you may retreat to the MELISA Client to command your server from anywhere in the world.

Grab your coffee ☕. It’s time to initialize the heart of the machine.

---