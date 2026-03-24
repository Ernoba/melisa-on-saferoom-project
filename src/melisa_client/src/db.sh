# File: db.sh
DB_PATH="$HOME/.config/melisa/registry"
mkdir -p "$(dirname "$DB_PATH")"
touch "$DB_PATH"

sed_wrapper() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

db_update_project() {
    local name=$1
    local path=$2
    if [ -z "$name" ] || [ -z "$path" ]; then return 1; fi
    
    # Pastikan path yang disimpan adalah absolute path tanpa trailing slash
    path=$(realpath "$path")
    
    # Hapus entri lama dengan nama yang sama
    sed_wrapper "\|^$name|d" "$DB_PATH"
    echo "$name|$path" >> "$DB_PATH"
}

db_get_path() {
    grep "^$1|" "$DB_PATH" | head -n 1 | cut -d'|' -f2
}

db_identify_by_pwd() {
    local current_dir=$(realpath "$PWD")
    local best_match_name=""
    local longest_path=0

    # Membaca database untuk mencari path yang merupakan parent dari $PWD saat ini
    while IFS='|' read -r name path; do
        # Cek apakah current_dir dimulai dengan path project
        if [[ "$current_dir" == "$path"* ]]; then
            # Ambil yang path-nya paling panjang (paling spesifik)
            if [ ${#path} -gt $longest_path ]; then
                longest_path=${#path}
                best_match_name="$name"
            fi
        fi
    done < "$DB_PATH"
    
    echo "$best_match_name"
}