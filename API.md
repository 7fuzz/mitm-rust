# MITM Real - API Documentation (Rust Migration)

MITM Real uses a Tauri-based architecture. The frontend (React + Vite) communicates directly with the Rust backend via Tauri commands.

- **Frontend**: React 19 + Vite
- **Backend**: Rust (Tauri)
- **Proxy Engine**: Rust (Custom Hyper/Tokio engine)
- **Database**: SQLite (via `rusqlite` and `tauri-plugin-sql`)

---

## 🏛️ Core & State

Global application state and synchronization.

| Command | Description |
| :--- | :--- |
| `sync_data` | **Bulk Fetch**: Retrieve History, Repeater, Environments, Variables, Replacements, Prefs, and Layout in one atomic transaction. |
| `update_prefs` | Save global preferences object (JSON). |
| `update_ui_layout` | Save UI layout state (sidebar width, split mode, etc). |
| `save_state` | Generic key-value store for arbitrary workspace state. |
| `upload_file` | Save binary data to local uploads folder and return path. |
| `get_proxy_status` | Get current proxy bindings and enabled status. |
| `toggle_proxy` | Enable or disable the proxy engine globally. |
| `update_network_settings` | Update listening ports/bindings. |
| `get_root_ca_pem` | Get the Root CA certificate for browser trust. |
| `regenerate_root_ca` | Wipe and recreate the CA certificate. |

---

## 🚦 Traffic & Intercept

Management of traffic flow and filtering.

| Command | Description |
| :--- | :--- |
| `update_state` | Update interception preferences (mode, ignored methods). |
| `update_filter_config` | Update global traffic filter rules (Allow/Block). |
| `resume_flow` | Resume or drop an intercepted request/response. |
| `traffic_captured` | **Event**: Emitted by backend when new traffic is logged. |

---

## 🔁 Repeater (Requests)

Manual request execution and workspace management.

| Command | Description |
| :--- | :--- |
| `execute_repeater_request` | Execute a request from the backend. Updates DB with response and saves to history. |
| `get_repeater_history` | Retrieve execution history for a specific request. |
| `clear_repeater_history` | Wipe history for a specific request. |
| `delete_repeater_history_item` | Delete a single history record. |
| `create_repeater_item` | Create a new request (often staged from History or Intercept). |
| `update_repeater_request` | Update request details (URL, Method, Headers, Body). |
| `delete_repeater_request` | Remove a request from the database. |
| `reorder_repeater_requests` | Update the display order of requests. |

---

## 📁 Repeater Groups (Collections)

Organization of Repeater items.

| Command | Description |
| :--- | :--- |
| `create_repeater_group` | Create a new collection. |
| `rename_repeater_group` | Rename an existing collection. |
| `delete_repeater_group` | Delete a group. |
| `reorder_repeater_groups` | Update the display order of collections. |
| `manage_group_assignment` | Link, unlink or move groups between environments. |

---

## 🌍 Environments & Variables

Dynamic variable system.

| Command | Description |
| :--- | :--- |
| `create_variable` | Create a new global variable. |
| `update_variable` | Update variable name or variants. |
| `delete_variable` | Delete a variable. |
| `create_environment` | Create a new server environment. |
| `delete_environment` | Delete an environment. |
| `set_active_environment` | Switch the active context. |

---

## 🔄 Automated Replacements

Rules for automatic traffic modification.

| Command | Description |
| :--- | :--- |
| `save_replacements_bulk` | Bulk upsert replacement rules. |
| `update_replacement_order` | Update the execution sequence. |
| `delete_replacement` | Remove a rule. |

---

## 📝 Data Structures

### multi-value headers
Headers are stored as `Vec<(String, String)>` to preserve duplicates like `Set-Cookie`.

### __form_data Abstraction
Used to manage complex forms in the UI before serialization.

```json
{
  "__form_data": [
    { "k": "user", "v": "admin", "type": "text" }
  ]
}
```
