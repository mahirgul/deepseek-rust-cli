#!/bin/bash
set -euo pipefail

# ─── DeepSeek Rust CLI - Linux/macOS Installer ─────────────────
# Usage: curl -fsSL <url> | bash
# Or:    bash install.sh

REPO="mahirgul/deepseek-rust-cli"
BIN_NAME="deepseek-rust-cli"
INSTALL_DIR="/usr/local/bin"

# ─── Color Output ──────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info()    { echo -e "${CYAN}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; }

# ─── Detect Platform ───────────────────────────────────────
detect_platform() {
    local os arch platform

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64|amd64)   platform="linux-x86_64" ;;
                aarch64|arm64)  platform="linux-aarch64" ;;
                *)              error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64|amd64)   platform="macos-x86_64" ;;
                arm64|aarch64)  platform="macos-aarch64" ;;
                *)              error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        *)
            error "Unsupported OS: $os"
            info "Windows users: run install.ps1 in PowerShell"
            exit 1
            ;;
    esac

    echo "$platform"
}

# ─── Check Dependencies ────────────────────────────────────
check_deps() {
    local missing=()

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        missing+=("curl or wget")
    fi
    if ! command -v tar &>/dev/null; then
        missing+=("tar")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        error "Missing required tools: ${missing[*]}"
        info "Install them first:"
        info "  Ubuntu/Debian: sudo apt install ${missing[*]}"
        info "  Fedora:        sudo dnf install ${missing[*]}"
        info "  macOS:         brew install ${missing[*]}"
        exit 1
    fi
}

# ─── Download ─────────────────────────────────────────────
download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL --retry 3 --retry-delay 2 -o "$dest" "$url"
    else
        wget -q --tries=3 --wait=2 -O "$dest" "$url"
    fi
}

# ─── Verify Checksum ──────────────────────────────────────
verify_checksum() {
    local file="$1" expected_sha="$2"
    local actual_sha

    if command -v sha256sum &>/dev/null; then
        actual_sha=$(sha256sum "$file" | cut -d' ' -f1)
    elif command -v shasum &>/dev/null; then
        actual_sha=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        warn "No sha256 tool found, skipping checksum verification"
        return 0
    fi

    if [ "$actual_sha" != "$expected_sha" ]; then
        error "Checksum verification FAILED!"
        error "  Expected: $expected_sha"
        error "  Got:      $actual_sha"
        exit 1
    fi
    success "Checksum verified"
}

# ─── Main ──────────────────────────────────────────────────
main() {
    echo ""
    echo -e "${BOLD}${CYAN}DeepSeek Rust CLI - Installer${NC}"
    echo ""

    check_deps

    platform=$(detect_platform)
    info "Detected platform: ${BOLD}$platform${NC}"

    # Get latest release
    info "Fetching latest release..."
    latest_tag=$(download "https://api.github.com/repos/$REPO/releases/latest" - 2>/dev/null | \
        grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$latest_tag" ]; then
        error "Could not find latest release for $REPO"
        exit 1
    fi
    info "Latest version: ${BOLD}$latest_tag${NC}"

    # Download URLs
    archive_name="$BIN_NAME-$platform.tar.gz"
    download_url="https://github.com/$REPO/releases/download/$latest_tag/$archive_name"
    checksum_url="${download_url}.sha256"

    # Create temp dir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    # Download archive
    info "Downloading $archive_name..."
    download "$download_url" "$tmpdir/$archive_name"

    # Download and verify checksum
    if expected_sha=$(download "$checksum_url" - 2>/dev/null | cut -d' ' -f1); then
        if [ -n "$expected_sha" ]; then
            verify_checksum "$tmpdir/$archive_name" "$expected_sha"
        fi
    else
        warn "Could not download checksum, skipping verification"
    fi

    # Extract
    info "Extracting..."
    tar -xzf "$tmpdir/$archive_name" -C "$tmpdir"

    # Install
    if [ -w "$INSTALL_DIR" ] || [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
        cp "$tmpdir/$BIN_NAME" "$INSTALL_DIR/"
    else
        info "Need sudo to install to $INSTALL_DIR"
        sudo cp "$tmpdir/$BIN_NAME" "$INSTALL_DIR/"
        sudo chmod +x "$INSTALL_DIR/$BIN_NAME"
    fi

    # Verify installation
    if command -v "$BIN_NAME" &>/dev/null; then
        success "Successfully installed ${BOLD}$BIN_NAME${NC} to ${BOLD}$INSTALL_DIR${NC}"
        echo ""
        info "Run '${BOLD}deepseek-rust-cli${NC}' to start"
    else
        warn "Binary installed but not in PATH."
        info "Add this to your shell config:"
        info "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi

    # ── Optional: set API key ─────────────────────────
    echo ""
    # Read from /dev/tty to work when piped (curl ... | bash)
    read -r -p "Set DEEPSEEK_API_KEY now? (y/N) " set_key < /dev/tty
    if [ "$set_key" = "y" ] || [ "$set_key" = "Y" ]; then
        read -r -p "Enter your DeepSeek API key: " api_key < /dev/tty
        if [ -n "$api_key" ]; then
            mkdir -p ~/.deep
            echo "DEEPSEEK_API_KEY=$api_key" >> ~/.deep/.env
            success "API key saved to ~/.deep/.env"
        fi
    fi

    echo ""
}

main
