#!/bin/bash
set -e

REPO="mahirgul/deepseek-rust-cli"
OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS_TYPE" == "linux" ]; then
    PLATFORM="linux-x86_64"
elif [ "$OS_TYPE" == "darwin" ]; then
    if [ "$ARCH" == "arm64" ]; then
        PLATFORM="macos-aarch64"
    else
        PLATFORM="macos-x86_64"
    fi
else
    echo "Unsupported OS: $OS_TYPE"
    exit 1
fi

LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_RELEASE" ]; then
    echo "Could not find latest release for $REPO"
    exit 1
fi

URL="https://github.com/$REPO/releases/download/$LATEST_RELEASE/deepseek-rust-cli-$PLATFORM.tar.gz"

echo "Downloading DeepSeek Rust CLI $LATEST_RELEASE for $PLATFORM..."
curl -L "$URL" -o deepseek-rust-cli.tar.gz

tar -xzf deepseek-rust-cli.tar.gz
sudo mv deepseek-rust-cli /usr/local/bin/

rm deepseek-rust-cli.tar.gz
echo "Successfully installed deepseek-rust-cli to /usr/local/bin/"
