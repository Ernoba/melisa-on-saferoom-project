#!/usr/bin/env bash
# ==============================================================================
# MELISA EXECUTION ENGINE
# Description: Handles remote code execution, project cloning, synchronization,
#              and artifact transfers via secure SSH pipelines.
# ==============================================================================

# --- UI Helpers (Minimalist & Clean) ---
# Define color variables explicitly to prevent empty variable evaluation errors
export BOLD='\e[1m'
export GREEN='\e[32m'
export RED='\e[31m'
export YELLOW='\e[33m'
export BLUE='\e[34m'
export RESET='\e[0m'

log_header()  { echo -e "\n${BLUE}::${RESET} ${BOLD}$1${RESET}"; }
log_stat()    { echo -e " ${GREEN}=>${RESET} $1: ${BOLD}$2${RESET}"; }
log_info()    { echo -e " ${BLUE}[INFO]${RESET} $1"; }
log_success() { echo -e " ${GREEN}[SUCCESS]${RESET} $1"; }
log_error()   { echo -ne " ${RED}[ERROR]${RESET} $1\n" >&2; }

# Source the local database module for path and project state resolution
source "$MELISA_LIB/db.sh"

# Validates that an active server connection is configured before proceeding
ensure_connected() {
    CONN=$(get_active_conn)
    if [ -z "$CONN" ]; then
        log_error "No active server connection found!"
        echo -e "  ${YELLOW}Tip:${RESET} Execute 'melisa auth add <name> <user@ip>' to register a server."
        exit 1
    fi
}

# ------------------------------------------------------------------------------
# REMOTE OPERATIONS (CONTAINER INTERACTION)
# ------------------------------------------------------------------------------

# Pipes a local script directly into a remote container's interpreter via SSH.
# Leaves zero footprint on the host machine.
exec_run() {
    ensure_connected
    local container=$1
    local file=$2
    
    if [ -z "$container" ] || [ -z "$file" ] || [ ! -f "$file" ]; then
        log_error "Usage: melisa run <container> <file>"
        exit 1
    fi
    
    # Dynamic interpreter resolution based on file extension
    local ext="${file##*.}"
    local interpreter="bash"
    if [ "$ext" == "py" ]; then interpreter="python3"; fi
    if [ "$ext" == "js" ]; then interpreter="node"; fi
    
    log_info "Executing '${BOLD}${file}${RESET}' inside '${container}' via server '${CONN}'..."
    # Stream the file content directly into the remote interpreter's STDIN
    cat "$file" | ssh "$CONN" "melisa --send $container $interpreter -"
}

# Compresses a local directory into a stream and extracts it directly inside the remote container.
exec_upload() {
    ensure_connected
    local container=$1
    local dir=$2
    local dest=$3
    
    if [ -z "$dest" ]; then
        log_error "Usage: melisa upload <container> <local_dir> <remote_dest>"
        exit 1
    fi
    
    log_info "Transferring '${dir}' to '${container}:${dest}' via server '${CONN}'..."
    # Tar stream execution: Compress locally, pipe over SSH, and extract remotely via MELISA
    tar -czf - -C "$dir" . | ssh "$CONN" "melisa --upload $container $dest"
}

# Uploads a script, executes it interactively (TTY), and cleans up afterward.
exec_run_tty() {
    ensure_connected
    local container=$1
    local file=$2
    
    if [ -z "$container" ] || [ -z "$file" ] || [ ! -f "$file" ]; then
        log_error "Usage: melisa run-tty <container> <file>"
        exit 1
    fi
    
    local filename=$(basename "$file")
    local dir=$(dirname "$file")
    local ext="${file##*.}"
    local interpreter="bash"
    [[ "$ext" == "py" ]] && interpreter="python3"
    [[ "$ext" == "js" ]] && interpreter="node"
    
    log_info "Provisioning artifact '${BOLD}${filename}${RESET}' in remote container..."
    
    # Securely upload the specific file to the container's /tmp directory
    if tar -czf - -C "$dir" "$filename" | ssh "$CONN" "melisa --upload $container /tmp" > /dev/null 2>&1; then
        log_success "Interactive session (TTY) initialized..."
        
        # Execute interactively (-t forces pseudo-tty allocation)
        ssh -t "$CONN" "melisa --send $container $interpreter /tmp/$filename"
        
        # Mandatory Cleanup Protocol
        ssh "$CONN" "melisa --send $container rm -f /tmp/$filename" > /dev/null 2>&1
        log_success "Execution cycle completed and artifacts purged."
    else
        log_error "Failed to transfer the artifact to the remote container."
    fi
}

# ------------------------------------------------------------------------------
# PROJECT ORCHESTRATION & SYNCHRONIZATION
# ------------------------------------------------------------------------------

# Visualizes the state of a directory after a synchronization event.
inspect_result() {
    local target=$1
    echo -e "\n\e[2m[Workspace State: $target]\e[0m"
    
    # Safely count entities, ignoring permission denied errors on restricted system files
    local files=$(find "$target" -type f 2>/dev/null | wc -l)
    local dirs=$(find "$target" -type d 2>/dev/null | wc -l)
    local size=$(du -sh "$target" 2>/dev/null | cut -f1)

    log_stat "Files" "$files"
    log_stat "Dirs"  "$dirs"
    log_stat "Size"  "$size"
    
    echo -e "\n\e[1;30mProject Topology (Depth 2):\e[0m"
    # Generate a clean, pseudo-tree visualization of the top two directory levels
    find "$target" -maxdepth 2 -not -path '*/.*' 2>/dev/null | sed "s|$target||" | sed 's|^/||' | grep -v "^$" | head -n 15 | sed 's/^/  /'
    
    [ "$files" -gt 15 ] && echo "  ..."
    echo ""
}

# Retrieves a project workspace from the master server via Git or Rsync.
exec_clone() {
    ensure_connected
    
    local project_name=""
    local force_clone=false

    # Robust argument parsing
    while [[ $# -gt 0 ]]; do
        case $1 in
            --force) force_clone=true; shift ;;
            *) [ -z "$project_name" ] && project_name=$1; shift ;;
        esac
    done

    if [ -z "$project_name" ]; then
        log_error "Usage: melisa clone <name> [--force]"
        exit 1
    fi

    log_header "Provisioning Workspace: $project_name"

    # --- ANTI-NESTING PROTOCOL ---
    # Prevents creating a folder inside a folder with the same name.
    local target_dir="./$project_name"
    if [ "$(basename "$PWD")" == "$project_name" ]; then
        target_dir="."
        log_info "Context Detected: Currently inside target directory. Syncing in place."
    fi

    if [ "$force_clone" = true ]; then
        log_info "Protocol: Force Overwrite (Direct Rsync)"
        local remote_path="~/$project_name/" 
        
        # Ensure the target directory exists if we aren't cloning in-place
        [ "$target_dir" != "." ] && mkdir -p "$target_dir"

        # Trailing slashes are CRITICAL for Rsync to copy contents rather than the directory itself
        if rsync -avz --progress "$CONN:$remote_path" "$target_dir/"; then
            local full_path="$(realpath "$target_dir")"
            db_update_project "$project_name" "$full_path"
            log_success "Synchronization complete at $full_path"
            inspect_result "$target_dir"
        else
            log_error "Rsync protocol failed. Verify server path and network connection."
        fi
    else
        log_info "Protocol: Version Control (Git Default)"
        
        # Git aborts if cloning into a non-empty directory. We trap this gracefully.
        if [ "$target_dir" == "." ] && [ "$(ls -A . 2>/dev/null)" ]; then
            log_error "Directory is not empty. Use '--force' for Rsync overwrite or navigate to an empty directory."
            exit 1
        fi

        if git clone "ssh://$CONN/opt/melisa/projects/$project_name" "$target_dir"; then
            local full_path="$(realpath "$target_dir")"
            db_update_project "$project_name" "$full_path"
            log_success "Repository successfully cloned to $full_path"
            inspect_result "$target_dir"
        else
            log_error "Git clone protocol failed."
        fi
    fi
}

# Pushes local changes to the remote repository and synchronizes untracked .env files.
exec_sync() {
    ensure_connected
    
    # 1. Identify the project context based on the current working directory
    local project_name=$(db_identify_by_pwd)
    
    if [ -z "$project_name" ]; then
        log_error "The current directory is not registered as a MELISA project workspace."
        exit 1
    fi

    # 2. Navigate to the absolute project root to ensure Git commands execute accurately
    local project_root=$(db_get_path "$project_name")
    cd "$project_root" || { log_error "Failed to access workspace root: $project_root"; exit 1; }

    local branch=$(git branch --show-current 2>/dev/null || echo "master")
    log_header "Synchronizing $project_name [Branch: $branch]"
    
    # Automated Stage & Commit sequence
    git add .
    git commit -m "melisa-sync: $(date +'%Y-%m-%d %H:%M')" --allow-empty > /dev/null
    
    log_info "Transmitting delta to host server..."
    if git push -f origin "$branch" 2>&1 | sed 's/^/  /'; then
        # Force the server-side to apply the physical update to the user's workspace
        ssh "$CONN" "melisa --update $project_name --force"
        
        # 3. Environment Synchronization
        # Untracked .env files are synced via Rsync utilizing the -R flag to preserve relative paths
        log_info "Synchronizing environment configurations (.env)..."
        local env_files=$(find . -maxdepth 2 -type f -name ".env")
        if [ -n "$env_files" ]; then
            echo "$env_files" | xargs -I {} rsync -azR "{}" "$CONN:~/$project_name/"
        fi
        
        log_success "Host server is now perfectly synchronized with local state."
    else
        log_error "Git push protocol failed. Verify network connectivity and remote configurations."
    fi
}

# Pulls the latest physical data from the host workspace to the local machine via Rsync.
exec_get() {
    ensure_connected
    
    local project_name=""
    local force_get=false
    
    # 1. Robust Argument Parsing
    while [[ $# -gt 0 ]]; do
        case $1 in
            --force) force_get=true; shift ;;
            *) [ -z "$project_name" ] && project_name=$1; shift ;;
        esac
    done

    # 2. Contextual Project Identification
    [ -z "$project_name" ] && project_name=$(db_identify_by_pwd)

    if [ -z "$project_name" ]; then
        log_error "Project context unknown. Usage: melisa get <name> [--force]"
        exit 1
    fi

    # 3. Path Resolution & Anti-Nesting Logic
    local local_path=$(db_get_path "$project_name")
    
    if [ -z "$local_path" ]; then
        # If not registered in the DB, check if the current folder matches the project name
        if [ "$(basename "$PWD")" == "$project_name" ]; then
            local_path="$(realpath .)"
        else
            local_path="$(realpath .)/$project_name"
        fi
    fi

    log_header "Retrieving Data for Workspace: $project_name"

    # 4. Rsync Execution Pipeline
    local remote_path="~/$project_name/"
    # Skip the .git directory to prevent local repo corruption
    local opts="-avz --progress --exclude='.git/'"
    
    if [ "$force_get" = true ]; then
        log_info "Protocol: Force Overwrite (Data Replacement)"
    else
        log_info "Protocol: Safe Sync (Ignoring existing files)"
        opts="$opts --ignore-existing"
    fi

    mkdir -p "$local_path"

    # Execute Rsync with trailing slashes to copy directory contents
    if rsync $opts "$CONN:$remote_path" "$local_path/"; then
        # 5. Update local registry metadata
        db_update_project "$project_name" "$local_path"
        
        log_success "Data retrieval completed at: $local_path"
        inspect_result "$local_path"
    else
        log_error "Rsync protocol failed. Verify that the workspace exists on the host server."
    fi
}

# Transparently forwards unrecognized commands directly to the MELISA host environment.
exec_forward() {
    ensure_connected
    log_header "Forwarding Payload: melisa $*"
    # -t enforces pseudo-tty allocation, allowing interactive remote commands
    ssh -t "$CONN" "melisa $*" 
}