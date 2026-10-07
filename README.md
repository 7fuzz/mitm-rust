# MITM Native - Pure Rust Intercepting Proxy & Debugger

A high-performance, developer-centric intercepting proxy and security debugging desktop application built in **pure native Rust** using `eframe` (egui) and `tokio`.

This native version completely eliminates WebKitGTK, DOM, and Node.js dependencies, reducing memory footprint from **350+ MB** to **~35–50 MB RSS** with instantaneous startup.

---

## 🚀 Quick Start

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install) (1.80+ recommended)
- **Linux GUI Build Dependencies** (Ubuntu / Debian):
  ```bash
  sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
  ```

### Run in Development

```bash
cargo run -p mitm-desktop
```

### Build for Production

```bash
cargo build --release -p mitm-desktop
```

The compiled standalone binary is placed at:
```bash
./target/release/mitm-desktop
```

---

## 🏗️ Workspace Architecture

The application is structured as a modular Cargo workspace:

```text
├── crates/
│   ├── mitm-core/          # Headless core proxy & persistence engine
│   │   ├── ca/             # Dynamic Root CA & host TLS certificate generation (rcgen)
│   │   ├── db/             # Embedded SQLite database (WAL mode, auto-migrations)
│   │   ├── proxy/          # Hyper + Rustls proxy with TLS SNI interception
│   │   ├── repeater/       # Manual HTTP request builder & executor
│   │   ├── fuzzer/         # Concurrent payload fuzzer engine
│   │   ├── ws/             # Bidirectional WebSocket proxy pipeline
│   │   ├── workspace/      # Workspaces, environments, and variable replacement
│   │   └── events.rs       # Broadcast event bus (CoreEvent)
│   │
│   └── mitm-desktop/       # Pure native desktop UI (eframe + egui_dock)
│       ├── assets/         # App icons and embedded resources
│       ├── state.rs        # Reactive UI state and event loop bridging
│       ├── views/          # Modular dockable views:
│       │   ├── traffic.rs      # Virtualized HTTP traffic table
│       │   ├── inspector.rs    # Headers, JSON, Raw, Hex dump inspector
│       │   ├── intercept.rs    # Live request/response breakpoint manager
│       │   ├── repeater.rs     # Multi-tab request builder
│       │   ├── fuzzer.rs       # Concurrent fuzzer matrix
│       │   ├── collections.rs  # Collapsible request tree
│       │   ├── websockets.rs   # WebSocket session & timeline viewer
│       │   └── settings.rs     # Proxy & CA certificate settings
│       └── main.rs         # Desktop application lifecycle & dock layout
```

---

## 🏃 Key Features

- **Blazing Fast Native UI**: Zero WebKit/Blink overhead. Rendered with hardware-accelerated immediate-mode GUI (`egui`).
- **Live Traffic Monitoring**: Virtualized table supporting high flow volumes with instant search, method, and status filtering.
- **Deep Flow Inspector**: Request and response tabs with formatted JSON viewer, raw text, and 3-column Hex dump (`Offset | Hex | ASCII`).
- **Live Interception & Breakpoints**: Pause requests or responses matching configured rules. Modify URLs, headers, or body before forwarding or dropping.
- **HTTP Repeater**: Multi-tab request construction with header and parameter editing, variable substitution (`{{var}}`), and full response analysis.
- **Concurrent Fuzzer**: Variable payload injection (`{{marker}}`) with configurable concurrency semaphore and live results matrix.
- **WebSocket Inspector**: Inspect active and historical WebSocket sessions with message timeline filtering.
- **Collections & Workspaces**: Organize saved requests hierarchically into collection folders with workspace switching.
- **SQLite Persistence**: Embedded SQLite storage operating in Write-Ahead Logging (WAL) mode for low-latency traffic streaming.

---

## 🔒 SSL Certificate Setup

To intercept HTTPS traffic:
1. Export or inspect the Root CA certificate from the **Settings** view.
2. Install and trust the certificate on your host machine or target mobile device.
3. Configure your HTTP proxy settings to point to `127.0.0.1:8080` (or your configured port).
