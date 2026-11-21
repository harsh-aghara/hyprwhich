#!/usr/bin/env bash
# hyprwhich - Wayland Layer-Shell Keybind HUD for Hyprland
# Installer script

set -euo pipefail

# ANSI Color Codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_banner() {
    cat << 'EOF'



█   █ █   █ ████  ████  █   █ █   █ ███  ███  █   █ 
█   █  █ █  █   █ █   █ █   █ █   █  █  █     █   █ 
█████   █   ████  ████  █ █ █ █████  █  █     █████ 
█   █   █   █     █  █  ██ ██ █   █  █  █     █   █ 
█   █   █   █     █   █ █   █ █   █ ███  ███  █   █ 



EOF
    echo -e "${BLUE}hyprwhich${NC} - Wayland Layer-Shell Keybind HUD for Hyprland\n"
}

print_help() {
    echo "Usage: ./install.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help            Show this help message and exit"
    echo "  --no-bind             Install binary only; skip Hyprland keybind configuration"
    echo "  --bin-dir <DIR>       Custom destination directory for the binary"
    echo "  --key <KEY>           Key to bind with SUPER/mainMod (default: H)"
    echo "  --dry-run             Show actions without copying files or modifying configs"
    echo ""
}

# CLI Argument Parsing
NO_BIND=false
CUSTOM_BIN_DIR=""
BIND_KEY="H"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            print_help
            exit 0
            ;;
        --no-bind)
            NO_BIND=true
            shift
            ;;
        --bin-dir)
            if [[ $# -lt 2 ]]; then
                echo -e "${RED}[✗] Error:${NC} --bin-dir requires a directory argument" >&2
                exit 1
            fi
            CUSTOM_BIN_DIR="$2"
            shift 2
            ;;
        --key)
            if [[ $# -lt 2 ]]; then
                echo -e "${RED}[✗] Error:${NC} --key requires a key name (e.g. H, SLASH, SPACE)" >&2
                exit 1
            fi
            BIND_KEY="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            echo -e "${RED}[✗] Error:${NC} Unrecognized option '$1'" >&2
            print_help
            exit 1
            ;;
    esac
done

print_banner

# Determine Target User and Home (handling sudo gracefully)
TARGET_USER="${SUDO_USER:-${USER:-$(id -un)}}"
TARGET_HOME="${HOME}"
if [[ "$EUID" -eq 0 && -n "${SUDO_USER:-}" ]]; then
    TARGET_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
fi

# Determine Binary Installation Directory
if [[ -n "$CUSTOM_BIN_DIR" ]]; then
    INSTALL_DIR="$CUSTOM_BIN_DIR"
elif [[ "$EUID" -eq 0 && -z "${SUDO_USER:-}" ]]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="${TARGET_HOME}/.local/bin"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

# 1. Locate or Compile Binary
BIN_SOURCE=""
if [[ -f "$SCRIPT_DIR/target/release/hyprwhich" ]]; then
    BIN_SOURCE="$SCRIPT_DIR/target/release/hyprwhich"
elif command -v cargo &>/dev/null; then
    echo -e "${CYAN}[i] Building release binary with cargo...${NC}"
    if [[ "$DRY_RUN" = false ]]; then
        (cd "$SCRIPT_DIR" && cargo build --release)
    fi
    BIN_SOURCE="$SCRIPT_DIR/target/release/hyprwhich"
else
    echo -e "${RED}[✗] Error:${NC} Precompiled binary not found and 'cargo' is not installed." >&2
    echo "    Please install Rust/Cargo (https://rustup.rs) or build hyprwhich first." >&2
    exit 1
fi

# 2. Install Binary & Symlinks
echo -e "${CYAN}[i] Installing binary to:${NC} $INSTALL_DIR/hyprwhich"
if [[ "$DRY_RUN" = false ]]; then
    mkdir -p "$INSTALL_DIR"
    cp "$BIN_SOURCE" "$INSTALL_DIR/hyprwhich"
    chmod +x "$INSTALL_DIR/hyprwhich"
    ln -sf "$INSTALL_DIR/hyprwhich" "$INSTALL_DIR/hyprwhich-gui"
fi
echo -e "${GREEN}[✓]${NC} Installed $INSTALL_DIR/hyprwhich"
echo -e "${GREEN}[✓]${NC} Created symlink $INSTALL_DIR/hyprwhich-gui"

# Also sync to Hyprland scripts dir if present
CONFIG_DIR="${XDG_CONFIG_HOME:-$TARGET_HOME/.config}/hypr"
HYPR_SCRIPTS="$CONFIG_DIR/scripts"
if [[ -d "$HYPR_SCRIPTS" ]]; then
    if [[ "$DRY_RUN" = false ]]; then
        cp "$INSTALL_DIR/hyprwhich" "$HYPR_SCRIPTS/hyprwhich"
        chmod +x "$HYPR_SCRIPTS/hyprwhich"
        ln -sf "$HYPR_SCRIPTS/hyprwhich" "$HYPR_SCRIPTS/hyprwhich-gui"
    fi
    echo -e "${GREEN}[✓]${NC} Updated Hyprland script: $HYPR_SCRIPTS/hyprwhich"
fi

# 3. Check PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo -e "\n${YELLOW}[!] Note:${NC} '$INSTALL_DIR' is not in your current PATH."
        echo -e "    Add it by appending this to your ~/.bashrc or ~/.zshrc:"
        echo -e "    ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
        ;;
esac

# 4. Hyprland Keybind Registration
if [[ "$NO_BIND" = true ]]; then
    echo -e "\n${CYAN}[i]${NC} Skipping keybind registration (--no-bind specified)."
elif [[ ! -d "$CONFIG_DIR" ]]; then
    echo -e "\n${YELLOW}[!]${NC} Hyprland configuration directory not found at $CONFIG_DIR."
    echo "    To manually bind hyprwhich, add this to your keybindings:"
    echo -e "    ${BOLD}bindd = \$mainMod, $BIND_KEY, Keybinds HUD (hyprwhich), exec, hyprwhich${NC}"
else
    # Check if hyprwhich is already registered anywhere in Hyprland configs
    EXISTING_FILE=""
    if EXISTING_MATCH=$(grep -rnE --include="*.conf" --include="*.lua" "(exec.*hyprwhich|exec_cmd.*hyprwhich)" "$CONFIG_DIR" 2>/dev/null | head -n 1); then
        EXISTING_FILE=$(echo "$EXISTING_MATCH" | cut -d: -f1)
    fi

    if [[ -n "$EXISTING_FILE" ]]; then
        echo -e "\n${GREEN}[✓]${NC} Keybind is already registered in:"
        echo -e "    ${BOLD}${EXISTING_FILE}${NC}"
    else
        # Determine best candidate target file
        TARGET_FILE=""
        CANDIDATES=(
            "$CONFIG_DIR/UserConfigs/UserKeybinds.lua"
            "$CONFIG_DIR/UserConfigs/UserKeybinds.conf"
            "$CONFIG_DIR/keybindings.conf"
            "$CONFIG_DIR/configs/Keybinds.lua"
            "$CONFIG_DIR/configs/Keybinds.conf"
            "$CONFIG_DIR/custom/keybinds.conf"
            "$CONFIG_DIR/conf.d/keybinds.conf"
            "$CONFIG_DIR/hyprland.lua"
            "$CONFIG_DIR/hyprland.conf"
        )

        for candidate in "${CANDIDATES[@]}"; do
            if [[ -f "$candidate" ]]; then
                TARGET_FILE="$candidate"
                break
            fi
        done

        # If not found in primary list, search for any bind file
        if [[ -z "$TARGET_FILE" ]]; then
            FOUND=$(find "$CONFIG_DIR" -maxdepth 2 -type f \( -name "*keybind*.conf" -o -name "*keybind*.lua" -o -name "*bind*.conf" \) 2>/dev/null | head -n 1)
            if [[ -n "$FOUND" ]]; then
                TARGET_FILE="$FOUND"
            elif [[ -f "$CONFIG_DIR/hyprland.conf" ]]; then
                TARGET_FILE="$CONFIG_DIR/hyprland.conf"
            elif [[ -f "$CONFIG_DIR/hyprland.lua" ]]; then
                TARGET_FILE="$CONFIG_DIR/hyprland.lua"
            else
                TARGET_FILE="$CONFIG_DIR/hyprland.conf"
            fi
        fi

        echo -e "\n${CYAN}[i] Registering keybind in:${NC} $TARGET_FILE"

        if [[ "$TARGET_FILE" == *.lua ]]; then
            # Lua configuration
            if [[ "$DRY_RUN" = false ]]; then
                cat << EOF >> "$TARGET_FILE"

-- hyprwhich: Wayland Layer-Shell Keybind HUD
hl.bind(mainMod .. " + $BIND_KEY", hl.dsp.exec_cmd("hyprwhich"))
EOF
            fi
            echo -e "${GREEN}[✓]${NC} Added keybind: ${BOLD}SUPER + $BIND_KEY -> hyprwhich${NC}"
        else
            # Standard Hyprlang configuration
            MOD_VAR="\$mainMod"
            if ! grep -rq "\$mainMod" "$CONFIG_DIR" 2>/dev/null; then
                MOD_VAR="SUPER"
            fi

            if [[ "$DRY_RUN" = false ]]; then
                cat << EOF >> "$TARGET_FILE"

# hyprwhich: Wayland Layer-Shell Keybind HUD
unbind = $MOD_VAR, $BIND_KEY
bindd = $MOD_VAR, $BIND_KEY, Keybinds HUD (hyprwhich), exec, hyprwhich
EOF
            fi
            echo -e "${GREEN}[✓]${NC} Added keybind: ${BOLD}$MOD_VAR + $BIND_KEY -> hyprwhich${NC}"
        fi

        # Live reload Hyprland if active
        if command -v hyprctl &>/dev/null && [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
            if [[ "$DRY_RUN" = false ]]; then
                hyprctl reload >/dev/null 2>&1 || true
            fi
            echo -e "${GREEN}[✓]${NC} Reloaded Hyprland configuration (keybind is live)"
        fi
    fi
fi

# 5. Success Summary
echo -e "\n${GREEN}${BOLD}Installation complete!${NC}"
echo -e "• Binary:       ${BOLD}$INSTALL_DIR/hyprwhich${NC}"
echo -e "• Test CLI:     ${BOLD}hyprwhich --list${NC}"
echo -e "• Launch HUD:   Press ${BOLD}SUPER + $BIND_KEY${NC} or run ${BOLD}hyprwhich${NC}\n"
