#!/bin/bash
# Build and run helper script for auto-wifi-manager

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse command line arguments
case "${1:-run}" in
    build)
        echo -e "${GREEN}Building in debug mode...${NC}"
        cargo build
        echo -e "${GREEN}✓ Build complete: target/debug/auto-wifi${NC}"
        ;;
    
    release)
        echo -e "${GREEN}Building in release mode (optimized)...${NC}"
        cargo build --release
        echo -e "${GREEN}✓ Release build complete: target/release/auto-wifi${NC}"
        ;;
    
    windows)
        echo -e "${GREEN}Building for Windows (64-bit)...${NC}"
        cargo build --release --target x86_64-pc-windows-gnu
        echo -e "${GREEN}✓ Windows build complete: target/x86_64-pc-windows-gnu/release/auto-wifi.exe${NC}"
        echo -e "${YELLOW}Note: Copy this .exe file along with chromedriver.exe to Windows${NC}"
        ;;
    
    run)
        echo -e "${GREEN}Running auto-wifi manager...${NC}"
        echo -e "${YELLOW}Note: ChromeDriver will be started automatically${NC}"
        cargo run
        ;;
    
    run-release)
        echo -e "${GREEN}Running optimized release build...${NC}"
        echo -e "${YELLOW}Note: ChromeDriver will be started automatically${NC}"
        if [ ! -f "target/release/auto-wifi" ]; then
            echo -e "${YELLOW}Release binary not found. Building...${NC}"
            cargo build --release
        fi
        ./target/release/auto-wifi
        ;;
    
    install)
        echo -e "${GREEN}Building release and installing...${NC}"
        cargo build --release
        
        # Try to install to ~/.local/bin first (no sudo)
        if [ -d "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin"; then
            cp target/release/auto-wifi "$HOME/.local/bin/"
            echo -e "${GREEN}✓ Installed to ~/.local/bin/auto-wifi${NC}"
            echo -e "${YELLOW}Make sure ~/.local/bin is in your PATH${NC}"
            echo -e "Add this to your ~/.bashrc if needed:"
            echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        else
            echo -e "${YELLOW}Installing to /usr/local/bin (requires sudo)...${NC}"
            sudo cp target/release/auto-wifi /usr/local/bin/
            echo -e "${GREEN}✓ Installed to /usr/local/bin/auto-wifi${NC}"
        fi
        ;;
    
    clean)
        echo -e "${YELLOW}Cleaning build artifacts...${NC}"
        cargo clean
        echo -e "${GREEN}✓ Clean complete${NC}"
        ;;
    
    check)
        echo -e "${GREEN}Checking code...${NC}"
        cargo check
        ;;
    
    test)
        echo -e "${GREEN}Running tests...${NC}"
        cargo test
        ;;
    
    help|--help|-h)
        echo "Auto WiFi Manager - Build & Run Helper"
        echo ""
        echo "Usage: ./build.sh [command]"
        echo ""
        echo "Commands:"
        echo "  build         Build in debug mode"
        echo "  release       Build in release mode (optimized)"
        echo "  windows       Build for Windows (cross-compile)"
        echo "  run           Run in debug mode (default)"
        echo "  run-release   Run optimized release build"
        echo "  install       Build release and install to system"
        echo "  clean         Clean build artifacts"
        echo "  check         Check code for errors"
        echo "  test          Run tests"
        echo "  help          Show this help message"
        echo ""
        echo "Examples:"
        echo "  ./build.sh              # Run in debug mode"
        echo "  ./build.sh release      # Build optimized binary"
        echo "  ./build.sh windows      # Build for Windows"
        echo "  ./build.sh install      # Install system-wide"
        echo ""
        echo "Features:"
        echo "  ✓ Automatic ChromeDriver management"
        echo "  ✓ Cross-platform desktop notifications"
        echo "  ✓ Automated PPPoE ID switching"
        ;;
    
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo "Run './build.sh help' for usage information"
        exit 1
        ;;
esac
