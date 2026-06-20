#!/bin/bash
# Build script for Voltshark firmware

set -e

echo "🔧 Building Voltshark firmware..."

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Install via rustup:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Add thumbv7em target if not present
if ! rustup target list | grep -q "thumbv7em-none-eabihf (installed)"; then
    echo "📦 Installing ARM Cortex-M4 target..."
    rustup target add thumbv7em-none-eabihf
fi

# Build for STM32F4
CARGO_TARGET_THUMBV7EM_NONE_EABIHF_RUNNER="probe-rs run --chip STM32F407VG" \
cargo build --target thumbv7em-none-eabihf --release

echo "✅ Build complete!"
echo ""
echo "Flash to device:"
echo "  cargo run --target thumbv7em-none-eabihf --release"
echo ""
echo "Or use probe-rs directly:"
echo "  probe-rs run --chip STM32F407VG target/thumbv7em-none-eabihf/release/voltshark"
