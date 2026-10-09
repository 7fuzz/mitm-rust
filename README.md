# MITM

An intercepting HTTP(S) and WebSocket proxy with a native desktop UI, written in Rust with `tokio`, `hyper`/`rustls` and `eframe` (egui). There's no webview, Node.js or npm.

## Run

Requirements: a recent stable Rust toolchain. On Debian/Ubuntu, also:

```bash
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

```bash
cargo run -p mitm-desktop                   # development
cargo build --release -p mitm-desktop       # ./target/release/mitm-desktop
```

## Install

```bash
cargo install --path crates/mitm-desktop    # puts mitm-desktop in ~/.cargo/bin
```

On Linux, add the launcher entry and icon (the session's `PATH` must include `~/.cargo/bin`):

```bash
install -Dm644 packaging/linux/mitm.desktop ~/.local/share/applications/mitm.desktop
install -Dm644 crates/mitm-desktop/assets/icons/icon.svg ~/.local/share/icons/hicolor/scalable/apps/mitm.svg
install -Dm644 crates/mitm-desktop/assets/icons/icon-512.png ~/.local/share/icons/hicolor/512x512/apps/mitm.png
```

Data lives in your local data directory under `mitm/`. On Linux, `~/.local/share/mitm/` holds `mitm.db` (history, workspaces, rules), `ca/` (the root CA), and `app.ron` (window layout and UI preferences).

Only one instance runs at a time. Launching a second one offers to switch to the running one or take over from it.

## HTTPS: trust the CA

The proxy listens on `127.0.0.1:8080` by default (add or change listeners in **Settings ▸ Proxy**). To read HTTPS traffic, the client has to trust the app's root CA:

1. Open **Settings ▸ Certificate** and use **Save certificate…** (PEM `.crt` or DER `.cer`), **Copy PEM**, or **Open folder**.
2. Trust it on the machine or device that sends the traffic:
   - **Arch / Fedora:** `sudo trust anchor --store mitm-ca.crt`
   - **Debian / Ubuntu:** copy to `/usr/local/share/ca-certificates/mitm-ca.crt`, then `sudo update-ca-certificates`
   - **macOS:** `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain mitm-ca.crt`
   - **Windows:** `certutil -addstore -f Root mitm-ca.cer`
   - **Firefox** keeps its own store: Settings ▸ Privacy & Security ▸ Certificates ▸ View Certificates ▸ Import.
   - **Android:** install the `.crt` as a user CA. Apps only trust it if their network security config allows user CAs.
3. Point the client's HTTP and HTTPS proxy at the listener.

**Regenerate CA** in the same section creates a new root; every device then has to trust the new one.

## Modules

Every module is a dock tab. Tabs can be split, moved or floated, and the layout is restored on the next launch. Open one from the tab bar's right-click menu, **Add Panel ▾**, or the command palette (`Ctrl + K`).

| Tab | What it does |
| --- | --- |
| Traffic | Live, virtualized flow table with search, filters and column layout. The Inspector shows headers, params, cookies and bodies (pretty, tree, form, preview, hex, raw), and can pop out into its own window. |
| Intercept | Pause requests or responses by rule, edit them, then forward or drop. |
| Rewrite | Rules that modify matching traffic automatically, with a live preview and a hit log. |
| Repeater | Request tabs with `{{var}}` substitution, run history and response view. |
| Collections | Saved requests in folders, per workspace, with pre/post requests and auto-extract rules. |
| Fuzzer | Sniper, pitchfork and cluster-bomb attacks over marked `{{variables}}`, with match rules and saved runs. |
| WebSockets | Sessions and frames, with a composer to send text, JSON or binary frames. |
| Webhooks | Local receiver with endpoints, signature verification, and replay to a URL. |
| Environments | Workspaces, environments and variables; import and export. |
| Settings | Proxy mode and listeners, certificate, appearance, shortcuts, data and a database browser. |
| Utilities ▸ | JSON Diff, JSON Tools, CVSS (3.1 and 4.0), Encoding, Hashing, HMAC signature. |

Keyboard shortcuts are listed in [shortcut.md](shortcut.md) and under **Settings ▸ Shortcuts**.

## Differences from the Tauri app (0.2.1)

These differences are intentional:

- **No webview.** Images preview inline. HTML, audio, video and PDF bodies show as source and open in an external app or save to a file.
- **Shortcuts use modifiers.** The `Ctrl + /` prefix keys and single-letter keys are gone, because they fire while typing. Tabs switch with `Alt + 1…9` and `Ctrl + Tab`, quick variables open on `Ctrl + E`, and Intercept uses `Ctrl + Enter` / `Ctrl + Backspace`. Shortcuts aren't customizable.
- **Docked tabs instead of pages and drawers.** Utilities are separate tabs, and rule editors sit in splits instead of modals.
- **One status flash instead of toast stacks.**
- **Repeater keeps its own environment**, separate from the workspace environments.
- **Bcrypt always uses a random salt.**
- **Unsaved fuzz runs are discarded on restart.** The fuzzer's setup is kept.

## Workspace layout

```text
crates/
├── mitm-core/          # Headless engine: no UI code
│   ├── lib.rs          # MitmCore, the API the app calls
│   ├── state.rs        # Shared runtime state and types
│   ├── events.rs       # CoreEvent bus the UI subscribes to
│   ├── instance.rs     # Single-instance guard
│   ├── proxy/          # hyper + rustls proxy, TLS interception, intercept and rewrite
│   ├── ca/             # Root CA and per-host certificates (rcgen)
│   ├── db/             # SQLite (WAL) storage and migrations
│   ├── http/           # Request bodies, cURL parsing, pre/post request steps
│   ├── repeater/  collections/  workspace/  fuzzer/  ws/  webhook/
│   └── utils/          # Encoding, hashing, JSON and CVSS helpers
└── mitm-desktop/       # eframe app
    ├── main.rs
    ├── app/            # MitmApp: commands, state, prefs, background jobs
    ├── shell/          # Window lifecycle: startup, instance conflict, DB recovery
    ├── ui/             # App-wide chrome: dock, top bar, palette, shortcuts, theme
    ├── features/       # One module per tab: state, view, prefs
    ├── widgets/        # Shared widgets (code view, diff, message pane, splits, …)
    └── util/           # File dialogs, copy-as formats
```

More docs: [WORKSPACE_FILE.md](WORKSPACE_FILE.md) (the import/export format), [API.md](API.md) (the `MitmCore` API), [DATABASE_SCHEMA.md](DATABASE_SCHEMA.md).
