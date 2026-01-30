<div align="center">

```
       ██╗  ██╗    ███╗   ██╗    ███████╗    ████████╗
       ╚██╗██╔╝    ████╗  ██║    ██╔════╝    ╚══██╔══╝
        ╚███╔╝     ██╔██╗ ██║    █████╗         ██║   
        ██╔██╗     ██║╚██╗██║    ██╔══╝         ██║   
       ██╔╝ ██╗    ██║ ╚████║    ███████╗       ██║   
       ╚═╝  ╚═╝    ╚═╝  ╚═══╝    ╚══════╝       ╚═╝   
```

<h3>🌍 Democratizing AI for Everyone 🌍</h3>



<div align="center">

**Making advanced AI accessible to everyone, everywhere.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-alpha-orange.svg)](https://github.com/yourusername/xNet)

[Features](#-features) • [Quick Start](#-quick-start) • [Architecture](#-architecture) • [Contributing](#-contributing)

</div>

---

## 🌟 Vision

xNet is a decentralized peer-to-peer network that makes cutting-edge AI models accessible to everyone—regardless of capital or infrastructure constraints.

### The Problem
- 💰 High-end AI models require expensive GPU infrastructure
- 🔒 Centralized services create data privacy concerns
- 🌍 Billions lack access to advanced AI capabilities

### Our Solution
A global network where:
- 🤖 **Anyone can USE** state-of-the-art AI models for free
- 🔗 **Anyone can SHARE** their idle compute resources
- 🌱 **Everyone CONTRIBUTES** to a decentralized AI commons

---

## ✨ Features

### 🎯 Core Functionality
- **Real AI Inference** - Local processing with Ollama integration
- **P2P Networking** - Decentralized node discovery and communication
- **Resource Sharing** - Contribute idle compute during downtime
- **Fair Rewards** - Earn credits for sharing resources

### 🚀 Advanced Capabilities
- **Federated Learning** - Privacy-preserving collaborative training
- **Distributed Inference** - Split large models across multiple nodes
- **Verification System** - Proof-of-inference to prevent fraud
- **Multi-Model Support** - Run various AI models (LLMs, vision, etc.)

### 🎨 User Experience
- **Modern Desktop App** - Beautiful Tauri-based interface
- **Real-time Dashboard** - Monitor network status and metrics
- **One-Click Setup** - Easy installation and configuration
- **Cross-Platform** - Windows, macOS, Linux support

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ ([install](https://rustup.rs/))
- Node.js 18+ ([install](https://nodejs.org/))
- Ollama ([install](https://ollama.ai/))

### Installation

**1. Install Ollama and download a model:**
```bash
# Linux/macOS
curl -L https://github.com/ollama/ollama/releases/download/v0.5.7/ollama-linux-amd64.tgz | tar -xz
./bin/ollama serve &
./bin/ollama pull tinyllama
```

**2. Clone and build xNet:**
```bash
git clone https://github.com/yourusername/xNet.git
cd xNet
cargo build --release
```

**3. Run the desktop app:**
```bash
cd desktop
npm install
npm run tauri dev
```

**4. Start your node:**
- Click "Start Node" in the app
- You're now part of the network! 🎉

---

## 📖 How It Works

```
┌─────────────────┐
│  Your Computer  │
│  (Idle Time)    │
└────────┬────────┘
         │
         ├─► Share compute resources
         │
    ┌────▼─────────────────────┐
    │   xNet P2P Network       │
    │  (Decentralized Nodes)   │
    └────┬─────────────────────┘
         │
         ├─► Process AI tasks
         ├─► Verify results
         └─► Earn credits
              ↓
    ┌─────────────────────┐
    │  Use AI Models      │
    │  (Free Access)      │
    └─────────────────────┘
```

### Architecture Highlights
- **Modular Design** - Clean separation (core, network, runtime)
- **Rust Backend** - Performance and safety
- **libp2p** - Battle-tested P2P networking
- **Tauri Frontend** - Lightweight desktop app
- **Ollama Runtime** - Local AI inference

📚 **Learn more:** [Architecture Documentation](docs/001_architecture_flow.md)

---

## 🎮 Usage

### Run AI Inference
```bash
# Using the API
curl -X POST http://localhost:3030/api/v1/task \
  -H "Content-Type: application/json" \
  -d '{"model": "tinyllama", "prompt": "Explain quantum computing"}'

# Using Python client
python3 demo_chat.py
```

### Join the Network
```bash
# Connect to a bootnode
./xnet --bootnode /ip4/1.2.3.4/tcp/4001/p2p/QmBootnode...
```

---

## 🛠️ Development

### Project Structure
```
xNet/
├── core/           # Core types, traits, domain logic
├── network/        # P2P networking (libp2p)
├── runtime/        # AI runtime wrapper (Ollama)
├── protocol/       # Protocol definitions
├── desktop/        # Tauri desktop application
└── docs/           # Documentation
```

### Build from source
```bash
# Build all workspace members
cargo build --release

# Run tests
cargo test --workspace

# Format code
cargo fmt --all
```

### Contributing
We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 🗺️ Roadmap

### ✅ Phase 1-5: Foundation (COMPLETED)
- [x] P2P networking infrastructure
- [x] Real AI inference with Ollama
- [x] Desktop application with modern UI
- [x] Federated learning framework
- [x] Verification system

### 🚧 Phase 6: Production (In Progress)
- [ ] Multi-node distributed inference
- [ ] Response streaming (SSE/WebSocket)
- [ ] Larger model support (Llama2, Mistral)
- [ ] Web dashboard for network monitoring
- [ ] Mobile app (React Native)

### 🔮 Future Vision
- [ ] Blockchain-based incentive layer
- [ ] Model marketplace
- [ ] Privacy-preserving computation (SMPC)
- [ ] GPU node support

📋 **Full roadmap:** [Implementation Plan](docs/000_implementation_plan.md)

---

## 🤝 Community

- 💬 **Discord:** [Join our community](https://discord.gg/xnet)
- 🐦 **Twitter:** [@xNetAI](https://twitter.com/xnetai)
- 📧 **Email:** hello@xnet.ai
- 🌐 **Website:** [xnet.ai](https://xnet.ai)

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

Built with amazing open-source technologies:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tauri](https://tauri.app/) - Desktop application framework
- [libp2p](https://libp2p.io/) - Peer-to-peer networking
- [Ollama](https://ollama.ai/) - Local AI runtime

Special thanks to all our [contributors](https://github.com/yourusername/xNet/graphs/contributors)!

---

<div align="center">

**⭐ Star us on GitHub — it helps!**

Made with ❤️ by the xNet community

[🚀 Get Started](#-quick-start) • [📖 Documentation](docs/) • [🐛 Report Bug](issues) • [💡 Request Feature](issues)

</div>
