#!/bin/bash
# ════════════════════════════════════════════════════════════════════════
# 🚀 Network Ghost v5.0 - Smart Router Build Script
# 🎯 Target: Google WiFi / OnHub (IPQ40xx/ARM Cortex-A7)
# 🔧 Cross-Compilation with extreme memory optimization
# ════════════════════════════════════════════════════════════════════════

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
PROJECT_NAME="network-ghost-v5"
VERSION="5.0.0"
PROFILE="release-router"

# Target configurations
TARGET_ARM64="aarch64-unknown-linux-musl"
TARGET_ARMV7="armv7-unknown-linux-musleabihf"

# Default target for Google WiFi (IPQ40xx is ARMv7)
DEFAULT_TARGET="$TARGET_ARMV7"

echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}   🌟 Network Ghost v${VERSION} - Smart Build System${NC}"
echo -e "${CYAN}   🎯 Optimized for Google WiFi (IPQ40xx/ARM Cortex-A7)${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo ""

# ════════════════════════════════════════════════════════════════════════
# Step 1: Environment Setup
# ════════════════════════════════════════════════════════════════════════

echo -e "${YELLOW}[1/8]${NC} Setting up build environment..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}❌ Rust not found!${NC}"
    echo -e "   Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

RUST_VERSION=$(rustc --version)
echo -e "${GREEN}✅ Rust: ${RUST_VERSION}${NC}"

# Add targets
echo -e "${YELLOW}   Adding cross-compilation targets...${NC}"
rustup target add "$TARGET_ARM64" 2>/dev/null || true
rustup target add "$TARGET_ARMV7" 2>/dev/null || true

# Install cross for easier cross-compilation
if ! command -v cross &> /dev/null; then
    echo -e "${YELLOW}   Installing cross-rs for cross-compilation...${NC}"
    cargo install cross --git https://github.com/cross-rs/cross 2>/dev/null || true
fi

# ════════════════════════════════════════════════════════════════════════
# Step 2: Select Target Architecture
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[2/8]${NC} Selecting target architecture..."
echo -e "   ${CYAN}1)${NC} ARMv7 (armhf) - Google WiFi/OnHub IPQ40xx, RPi3"
echo -e "   ${CYAN}2)${NC} ARM64 (aarch64) - Google WiFi OnHub (newer), RPi4"
echo ""

# Auto-detect or use default
if [ -n "$ROUTER_ARCH" ]; then
    if [ "$ROUTER_ARCH" = "arm64" ] || [ "$ROUTER_ARCH" = "aarch64" ]; then
        TARGET="$TARGET_ARM64"
        ARCH_NAME="ARM64 (aarch64)"
    else
        TARGET="$TARGET_ARMV7"
        ARCH_NAME="ARMv7 (IPQ40xx)"
    fi
else
    TARGET="$DEFAULT_TARGET"
    ARCH_NAME="ARMv7 (Google WiFi IPQ40xx)"
fi

echo -e "${GREEN}✅ Target: ${TARGET} (${ARCH_NAME})${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 3: Clean Previous Build
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[3/8]${NC} Cleaning previous builds..."

cargo clean --target "$TARGET" 2>/dev/null || true
rm -rf "target/${TARGET}/${PROFILE}"
rm -rf "target/${TARGET}/release"
rm -f "${PROJECT_NAME}-"*".tar.gz"
rm -f "${PROJECT_NAME}-arm"*

echo -e "${GREEN}✅ Clean complete${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 4: Set Optimizations for IPQ40xx
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[4/8]${NC} Configuring extreme optimizations..."

# Platform-specific flags
if [ "$TARGET" = "$TARGET_ARMV7" ]; then
    # IPQ40xx = ARM Cortex-A7 quad-core
    export RUSTFLAGS="-C target-cpu=cortex-a7 -C link-arg=-s -C opt-level=z -C codegen-units=1"
    echo -e "   ${CYAN}CPU: Cortex-A7 (IPQ40xx)${NC}"
elif [ "$TARGET" = "$TARGET_ARM64" ]; then
    # ARM64 generic
    export RUSTFLAGS="-C target-cpu=cortex-a53 -C link-arg=-s -C opt-level=z -C codegen-units=1"
    echo -e "   ${CYAN}CPU: Cortex-A53 (ARM64)${NC}"
fi

echo -e "   ${CYAN}Profile: ${PROFILE}${NC}"
echo -e "   ${CYAN}Opt-level: z (minimum size)${NC}"
echo -e "   ${CYAN}LTO: fat (maximum optimization)${NC}"
echo -e "   ${CYAN}Panic: abort (no unwinding)${NC}"
echo -e "   ${CYAN}Strip: symbols${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 5: Build Binary
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[5/8]${NC} Compiling with extreme optimizations..."
echo ""

BUILD_START=$(date +%s)

# Build command with cross
if command -v cross &> /dev/null; then
    echo -e "${CYAN}   Using cross-rs for cross-compilation...${NC}"
    cross build \
        --target "$TARGET" \
        --profile "$PROFILE" \
        --features "full,router-optimized"
else
    echo -e "${YELLOW}   Using cargo (native compilation)...${NC}"
    
    # Set environment for musl cross-compilation
    if [ "$TARGET" = "$TARGET_ARMV7" ]; then
        export CC_armv7_unknown_linux_musleabihf="arm-linux-gnueabihf-gcc"
        export CXX_armv7_unknown_linux_musleabihf="arm-linux-gnueabihf-g++"
        export AR_armv7_unknown_linux_musleabihf="arm-linux-gnueabihf-ar"
    else
        export CC_aarch64_unknown_linux_musl="aarch64-linux-gnu-gcc"
        export CXX_aarch64_unknown_linux_musl="aarch64-linux-gnu-g++"
        export AR_aarch64_unknown_linux_musl="aarch64-linux-gnu-ar"
    fi
    
    cargo build \
        --target "$TARGET" \
        --profile "$PROFILE" \
        --features "full,router-optimized"
fi

BUILD_END=$(date +%s)
BUILD_TIME=$((BUILD_END - BUILD_START))

echo -e "${GREEN}✅ Build complete (${BUILD_TIME}s)${NC}"

# ════════════════════════════════════════════════════════════════════════
# ════════════════════════════════════════════════════════════════════════
# Step 6: Extreme Optimization (Strip + UPX Compression)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[6/8]${NC} Performing Multi-Layer Optimization..."

# ۱. پیدا کردن فایل باینری (هوشمند)
BINARY_PATH="target/${TARGET}/${PROFILE}/network-ghost"
if [ ! -f "$BINARY_PATH" ]; then
    BINARY_PATH="target/${TARGET}/release/network-ghost"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Binary not found! Build failed.${NC}"
    exit 1
fi

# ۲. آماده‌سازی نام خروجی
ARCH_SHORT=$(echo "$TARGET" | cut -d'-' -f1)
OUTPUT_NAME="${PROJECT_NAME}-${VERSION}-${ARCH_SHORT}-ipq40xx"
cp "$BINARY_PATH" "./${OUTPUT_NAME}"
chmod +x "./${OUTPUT_NAME}"

# ۳. لایه اول: Stripping (پاکسازی زواید متنی و دیباگ)
# این کار مثل اینه که بارهای اضافه هواپیما رو بریزی بیرون
echo -e "${CYAN}   🚀 Layer 1: Stripping debug symbols...${NC}"
case "$TARGET" in
    *armv7*)
        arm-linux-gnueabihf-strip "./${OUTPUT_NAME}" 2>/dev/null || strip "./${OUTPUT_NAME}" 2>/dev/null || true
        ;;
    *aarch64*)
        aarch64-linux-gnu-strip "./${OUTPUT_NAME}" 2>/dev/null || strip "./${OUTPUT_NAME}" 2>/dev/null || true
        ;;
esac

# ۴. لایه دوم: Ultra-Compression (فشرده‌سازی با UPX)
# این کار مثل اینه که بدنه هواپیما رو جمع‌وجور و آیرودینامیک کنی
if command -v upx > /dev/null; then
    echo -e "${MAGENTA}   🚀 Layer 2: Applying UPX Ultra-Brute compression...${NC}"
    upx --best --ultra-brute "./${OUTPUT_NAME}" > /dev/null 2>&1
else
    echo -e "${YELLOW}   ⚠️ UPX not found. Skipping Layer 2 compression.${NC}"
fi

# ۵. محاسبه حجم نهایی و گزارش پیروزی
BINARY_SIZE=$(stat -c%s "./${OUTPUT_NAME}" 2>/dev/null || stat -f%z "./${OUTPUT_NAME}")
BINARY_SIZE_KB=$((BINARY_SIZE / 1024))
BINARY_SIZE_MB=$(echo "scale=2; $BINARY_SIZE / 1048576" | bc 2>/dev/null || echo "$((BINARY_SIZE / 1048576))")

echo -e "${GREEN}✅ Optimization Complete!${NC}"
echo -e "   Final Size: ${CYAN}${BINARY_SIZE_KB} KB${NC} (~${BINARY_SIZE_MB} MB)"

# ════════════════════════════════════════════════════════════════════════
# Step 7: Create Deployment Package
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[7/8]${NC} Creating deployment package..."

PACKAGE_DIR="package/${PROJECT_NAME}-${VERSION}"
mkdir -p "$PACKAGE_DIR/bin"
mkdir -p "$PACKAGE_DIR/config"
mkdir -p "$PACKAGE_DIR/scripts"

# Copy binary
cp "./${OUTPUT_NAME}" "$PACKAGE_DIR/bin/network-ghost"

# Copy configuration
cp config/config.toml "$PACKAGE_DIR/config/" 2>/dev/null || true
cp p-list-multicdn.txt "$PACKAGE_DIR/config/" 2>/dev/null || true

# Copy scripts
cp setup-router.sh "$PACKAGE_DIR/scripts/" 2>/dev/null || true
cp build-router.sh "$PACKAGE_DIR/scripts/" 2>/dev/null || true

# Create README
cat > "$PACKAGE_DIR/README.txt" << EOF
═══════════════════════════════════════════════════════════════
   🌟 Network Ghost v${VERSION} - Phantom Edition
   🎯 Optimized for IPQ40xx (Google WiFi/OnHub)
═══════════════════════════════════════════════════════════════

Target: ${ARCH_NAME} (${TARGET})
Binary Size: ${BINARY_SIZE_MB}MB
Profile: ${PROFILE} (opt-level=z, LTO=fat)

───────────────────────────────────────────────────────────────
📦 Installation:
───────────────────────────────────────────────────────────────

1. Copy to router:
   scp -r ${PROJECT_NAME}-${VERSION} root@ROUTER_IP:/tmp/

2. Run setup:
   ssh root@ROUTER_IP
   cd /tmp/${PROJECT_NAME}-${VERSION}/scripts
   chmod +x setup-router.sh
   ./setup-router.sh

───────────────────────────────────────────────────────────────
🚀 Advanced Features (v5.0):
───────────────────────────────────────────────────────────────

✅ IPQ40xx Hardware Offload Hints
✅ Adaptive Buffer Management (RAM 512MB)
✅ Thermal Throttling Prevention
✅ CPU Core Affinity (4-core Cortex-A7)
✅ Smart Filter Detection (AI/ML Bypass)
✅ Matryoshka 20-Layer Chain
✅ Anti-AI DPI System
✅ TLS Fragmentation Detection
✅ Port Hopping (Dynamic)
✅ Dashboard on port 9090
✅ Kernel-Level DAE/eBPF Integration
✅ Multi-CDN Failover

───────────────────────────────────────────────────────────────
📊 Dashboard: http://ROUTER_IP:9090
───────────────────────────────────────────────────────────────
EOF

# Create tarball
cd package
tar czf "../${OUTPUT_NAME}.tar.gz" "${PROJECT_NAME}-${VERSION}"
cd ..

# Generate checksum
sha256sum "./${OUTPUT_NAME}.tar.gz" > "./${OUTPUT_NAME}.tar.gz.sha256"

# ════════════════════════════════════════════════════════════════════════
# Step 8: Summary
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  ✅ Network Ghost v${VERSION} Build Successful!              ║${NC}"
echo -e "${GREEN}║  🎯 IPQ40xx Optimized                                        ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  📦 Binary:      ${CYAN}./${OUTPUT_NAME}${NC}"
echo -e "  📦 Package:     ${CYAN}./${OUTPUT_NAME}.tar.gz${NC}"
echo -e "  ⚖️  Size:        ${CYAN}${BINARY_SIZE_MB}MB${NC} (${BINARY_SIZE_KB}KB)"
echo -e "  🎯 Target:      ${CYAN}${TARGET}${NC}"
echo -e "  ⏱️  Build Time:  ${CYAN}${BUILD_TIME}s${NC}"
echo ""
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}📡 Deployment Commands:${NC}"
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${CYAN}# Copy to router:${NC}"
echo -e "  scp ./${OUTPUT_NAME}.tar.gz root@ROUTER_IP:/tmp/"
echo ""
echo -e "  ${CYAN}# On router:${NC}"
echo -e "  cd /tmp && tar xzf ${OUTPUT_NAME}.tar.gz"
echo -e "  cd ${PROJECT_NAME}-${VERSION}/scripts && ./setup-router.sh"
echo ""
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🔧 New IPQ40xx Features:${NC}"
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${GREEN}✅${NC} Hardware Offload Hints for NAT/Firewall"
echo -e "  ${GREEN}✅${NC} Adaptive Buffer Management (auto-adjusts for 512MB RAM)"
echo -e "  ${GREEN}✅${NC} Thermal Throttling Prevention"
echo -e "  ${GREEN}✅${NC} CPU Core Affinity (4-core Cortex-A7)"
echo -e "  ${GREEN}✅${NC} Smart Filter Detection & Auto-Bypass"
echo ""
echo -e "${GREEN}🎉 Phantom Build Complete! Ready for deployment.${NC}"
echo ""

# Cleanup
rm -f "./${OUTPUT_NAME}"
rm -rf package
