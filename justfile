# App4.Dog Game Development Commands
# Interactive pet training game built with Quasar/Capacitor and Rust/Bevy WASM

# Default recipe - show available commands
default:
    @just --list

# Development commands
dev:
    @echo "🚀 Starting Quasar development server..."
    pnpm run dev

# Development with WASM rebuild
dev-wasm: rebuild-wasm
    @echo "🚀 Starting Quasar development server with fresh WASM..."
    pnpm run dev

build:
    @echo "🏗️ Building Quasar application..."
    pnpm run build

# Game engine commands
build-wasm:
    @echo "🦀 Building Rust game engine to WASM..."
    chmod +x scripts/build-wasm.sh
    ./scripts/build-wasm.sh

# Quick WASM rebuild for development
rebuild-wasm: clean-wasm build-wasm
    @echo "🔄 WASM rebuilt - refresh browser to see changes"

wasm-dev:
    @echo "🔄 Building WASM in development mode..."
    WASM_MODE=dev ./scripts/build-wasm.sh

wasm-release:
    @echo "🚀 Building WASM in release mode..."
    WASM_MODE=release ./scripts/build-wasm.sh

# Mobile development
dev-android: build-wasm
    @echo "📱 Starting Android development..."
    pnpm run build
    npx cap sync android
    npx cap run android

# Build Android APK using Docker (no local Android SDK required)
build-android-docker: build-wasm build
    @echo "🐳 Building Android APK using Docker..."
    ./build-android.sh

dev-ios: build-wasm
    @echo "📱 Starting iOS development..."
    pnpm run build
    npx cap sync ios
    npx cap run ios

# Capacitor mobile commands
cap-sync: build
    @echo "🔄 Syncing with Capacitor..."
    npx cap sync

cap-open-android:
    @echo "📱 Opening Android Studio..."
    npx cap open android

cap-open-ios:
    @echo "📱 Opening Xcode..."
    npx cap open ios


# Testing and linting
test:
    @echo "🧪 Running tests..."
    pnpm run test

lint:
    @echo "🔍 Running ESLint..."
    pnpm run lint

format:
    @echo "✨ Formatting code..."
    pnpm run format

# Git and deployment
commit: lint format
    @echo "📝 Staging and committing changes..."
    git add .
    git status

push: commit
    @echo "🚀 Pushing to GitHub..."
    git push origin main

# Development setup
install:
    @echo "📦 Installing dependencies..."
    pnpm install
    @echo "🦀 Installing Rust tools..."
    rustup target add wasm32-unknown-unknown
    cargo install wasm-pack
    @echo "🗜️ Installing WASM optimizer..."
    npm install -g binaryen
    @echo "✅ Development environment ready!"

# Clean commands
clean:
    @echo "🧹 Cleaning build artifacts..."
    rm -rf dist/
    rm -rf public/game-engine/
    rm -rf game-engine/pkg/
    rm -rf game-engine/target/

clean-wasm:
    @echo "🧹 Cleaning WASM artifacts..."
    rm -rf public/game-engine/
    rm -rf game-engine/pkg/

clean-all: clean
    @echo "🧹 Cleaning all dependencies..."
    rm -rf node_modules/
    rm -rf game-engine/target/

# Help and information
info:
    @echo "📋 App4.Dog Game Project Information:"
    @echo "  Frontend: Quasar (Vue 3 + TypeScript) + Capacitor"
    @echo "  Game Engine: Rust/Bevy compiled to WASM"
    @echo "  Mobile: Android/iOS via Capacitor"
    @echo "  Assets: Migrated from puppyplay-godot-droid"
    @echo ""
    @echo "🎯 Purpose: Interactive pet training game for dogs"
    @echo "🐾 Players: Real pets interact with anthropomorphic critters"
    @echo ""
    @echo "📁 Key directories:"
    @echo "  src/          - Vue/Quasar frontend"
    @echo "  game-engine/  - Rust/Bevy game logic"
    @echo "  public/assets/ - Game sprites, audio, fonts"
    @echo "  scripts/      - Build and deployment scripts"

# Development workflow
dev-full: clean install build-wasm dev

# Release workflow
release: clean install wasm-release build

# CI/CD friendly build commands
ci-build-wasm:
    @echo "🦀 Building WASM for CI..."
    @chmod +x scripts/build-wasm.sh
    @WASM_MODE=release ./scripts/build-wasm.sh

ci-build: ci-build-wasm
    @echo "🏗️ Building application for CI..."
    @pnpm run build

ci-docker-build: ci-build
    @echo "🐳 Building Docker image..."
    @docker build -t app4dog-game:latest .

ci-docker-run: ci-docker-build
    @echo "🚀 Running Docker container..."
    @docker run -d -p 8080:80 --name app4dog-game app4dog-game:latest

# Development commands - no CI dependencies  
dev-build: build-wasm build
    @echo "✅ Development build complete!"

# Quick local deployment (legacy - now prefer CI/CD)
deploy-local: dev-build
    @echo "☁️ Deploying locally built version..."
    @npx wrangler deploy

# Smart deployment - detects and installs what's needed (legacy - slow)
deploy-full:
    @echo "🚀 Starting full deployment (slow - prefer CI/CD)..."
    @# Check and install Rust if needed
    @if ! command -v rustc &> /dev/null; then \
        echo "🦀 Installing Rust toolchain..."; \
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
        . ~/.cargo/env; \
        ~/.cargo/bin/rustup target add wasm32-unknown-unknown; \
    else \
        echo "✅ Rust already installed"; \
        . ~/.cargo/env 2>/dev/null || true; \
    fi
    @# Check and install wasm-pack if needed
    @if ! command -v wasm-pack &> /dev/null && ! [ -f ~/.cargo/bin/wasm-pack ]; then \
        echo "📦 Installing wasm-pack..."; \
        . ~/.cargo/env 2>/dev/null || true; \
        ~/.cargo/bin/cargo install wasm-pack || curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh; \
    else \
        echo "✅ wasm-pack already installed"; \
    fi
    @# Install dependencies if needed
    @if [ ! -d "node_modules" ]; then \
        echo "📦 Installing dependencies..."; \
        pnpm install; \
    else \
        echo "✅ Dependencies already installed"; \
    fi
    @# Build WASM
    @echo "🦀 Building WASM..."
    @chmod +x scripts/build-wasm.sh
    @./scripts/build-wasm.sh
    @# Build app
    @echo "🏗️ Building application..."
    @pnpm run build
    @# Deploy
    @echo "☁️ Deploying to Cloudflare Worker..."
    @npx wrangler deploy
    @echo "✅ Deployment complete!"

# Default deploy now uses pre-built artifacts from CI
deploy:
    @echo "🚀 Quick deployment using pre-built artifacts..."
    @if [ ! -d "dist/spa" ]; then \
        echo "❌ No pre-built artifacts found. Run 'just ci-build' or use 'just deploy-local'"; \
        exit 1; \
    fi
    @echo "☁️ Deploying to Cloudflare Worker..."
    @npx wrangler deploy
    @echo "✅ Deployment complete!"