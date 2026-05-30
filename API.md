# MITM Real - API Documentation (Rust Migration)

MITM Real is migrating to a Tauri-based architecture. In this version, the frontend (React + Vite) communicates directly with the Rust backend via Tauri commands and potentially a local HTTP/SSE interface for the proxy engine.

- **Frontend**: React 19 + Vite
- **Backend**: Rust (Tauri)
- **Proxy Engine**: Rust (Integrated or via specialized crates)

---

## 🏛️ Core & State
Global application state and intercept control.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_state` | Tauri | Retrieve global application state (preferences, queue). |
| `update_state` | Tauri | Update global preferences and application state. |
| `get_cert` | Tauri | Download the Root CA certificate (`.pem`). |
| `resume_flow` | Tauri | Resume or drop an intercepted request. |

---

## 🚦 Traffic & History
Management of the HTTP history log.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_history` | Tauri | Fetch all historical traffic flows. |
| `clear_history` | Tauri | Clear the entire history log. |
| `delete_history_item`| Tauri | Delete a single history item by ID. |
| `traffic_events` | SSE/Event| Listen for real-time traffic updates via Tauri Events or SSE. |

---

## 🔁 Repeater (Requests)
Manual request execution and persistent workspace items.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `execute_repeat` | Tauri | Execute a manual request (interpolates variables). |
| `create_repeater_item`| Tauri | Create a new request in the Repeater workspace. |
| `get_repeater_items` | Tauri | Fetch saved Repeater items. |
| `update_repeater_items`| Tauri | Bulk update or reorder Repeater items. |
| `update_repeater_item` | Tauri | Update a specific Repeater item. |
| `delete_repeater_item` | Tauri | Remove a request from the Repeater workspace. |
| `import_requests` | Tauri | Import requests from Postman or MITM Real JSON exports. |

### Repeater History
Individual execution history for specific Repeater items.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_repeater_history`| Tauri | Get execution history for a specific request. |
| `clear_repeater_history`| Tauri | Clear history for a specific request. |

---

## 📁 Repeater Groups (Collections)
Organization of Repeater items into collections.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_groups` | Tauri | Fetch all Repeater groups. |
| `create_group` | Tauri | Create a new collection group. |
| `rename_group` | Tauri | Rename a collection group. |
| `delete_group` | Tauri | Delete a group and all its contained requests. |
| `assign_groups` | Tauri | Link, unlink or move groups between environments. |
| `reorder_groups` | Tauri | Update the display order of groups. |

---

## 🌍 Environments & Variables
Dynamic variable system and server environments.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_variables` | Tauri | Fetch variables for an environment. |
| `create_variable` | Tauri | Create a new variable. |
| `update_variable` | Tauri | Update variable name, variants, or active index. |
| `delete_variable` | Tauri | Delete a variable. |
| `bulk_update_variables`| Tauri | Bulk update multiple variables. |
| `set_environment` | Tauri | Create or set the active environment. |
| `rename_environment` | Tauri | Rename an environment. |
| `delete_environment` | Tauri | Delete an environment. |

---

## 🔄 Automated Replacements
Rules for automatic traffic modification.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `get_replacements` | Tauri | Fetch all replacement rules. |
| `upsert_replacement` | Tauri | Create or update replacement rules. |
| `reorder_replacements`| Tauri | Update the execution order of rules. |
| `delete_replacement` | Tauri | Delete a replacement rule. |

---

## 📤 File Uploads
Persistence for files used in `multipart/form-data` requests.

| Command / Endpoint | Method | Description |
| :--- | :--- | :--- |
| `upload_file` | Tauri | Upload a file to the application storage. |

---

## 📝 Request/Response Structures

### __form_data Abstraction
Used in the Repeater and Intercept views to manage complex forms.
```json
{
  "__form_data": [
    {
      "k": "username",
      "v": "admin",
      "type": "text",
      "contentType": ""
    },
    {
      "k": "profile_pic",
      "v": "uuid-of-stored-file.png",
      "type": "file",
      "fileName": "avatar.png",
      "contentType": "image/png"
    }
  ]
}
```

### Interpolation Engine
Variables in the backend are interpolated using two syntaxes:
- `{{variable_name}}`: Standard workspace variables.
- `[[today+1]]`: Dynamic time-based variables (today, yesterday, tomorrow with +/- offsets).
