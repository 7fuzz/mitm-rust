# mitm-core API

`mitm-core` is the headless engine. It has no UI code, and `mitm-desktop` is its only consumer. There are two layers:

- **`MitmCore`** (`crates/mitm-core/src/lib.rs`) owns the shared state, the root CA and the event bus. Anything that touches running servers (proxy listeners, interceptor, rewrite, WebSocket and webhook servers, the fuzzer) goes through it.
- **Module functions** ending in `_db` take the database path and run synchronously against SQLite. The desktop calls them for plain storage work (history pages, repeater, collections, workspaces), often from a background job.

Errors are `Result<_, String>` with a message meant for the UI.

## Lifecycle and events

| Item | Notes |
| --- | --- |
| `MitmCore::init()` | Opens or migrates the DB in the local data dir (`mitm/mitm.db`), loads the CA, restores saved settings and discards unsaved fuzz runs. |
| `MitmCore::init_with_events(bus)` | Same, on a bus the caller already subscribed to, so `DbMigrationProgress` is observable during startup. |
| `core.event_bus.subscribe()` | `tokio::sync::broadcast` receiver of `CoreEvent`. |
| `factory_reset()` | Stops the listeners, resets the DB, and emits `TrafficCleared`. |
| `instance::try_claim(handlers)` | Single-instance lock. Returns `Primary`, `Conflict(other)`, or `Unguarded` when the guard port is held by something else. |

`CoreEvent` (`events.rs`):

| Variant | When |
| --- | --- |
| `TrafficCaptured(TrafficCapturedEvent)` | A flow was stored. |
| `TrafficUpdated { id, status, size, duration_ms }` | A flow got its response. |
| `TrafficCleared` | History was cleared. |
| `InterceptPrompt(PendingFlowPayload)` / `InterceptResolved { flow_id }` | A flow paused in, or left, the interceptor. |
| `FuzzResult { run_id, row }` / `FuzzFinished { run_id, completed, total, stopped }` | Fuzzer progress. A row is sent only after it's stored. |
| `WsFrameReceived { session_id, frame }` | A WebSocket open/close (`WebSocketConn`) or message (`WebSocketMessage`), as JSON. |
| `WebhookDelivery(WebhookDeliveryCapturedEvent)` | The receiver stored a delivery. |
| `DbMigrationProgress(MigrationProgressPayload)` | A startup migration step. |

## Proxy and listeners

| Method | Notes |
| --- | --- |
| `proxy_mode()` / `set_proxy_mode(mode)` | `On`, `Off`, `BlockClient`, `BlockAll`. Saved, and starts or stops listeners to match. |
| `get_listener_configs()` / `set_listener_configs(configs)` | Saved listener list (id, label, address, enabled). |
| `start_all_listeners()` / `stop_all_listeners()` | Returns the summaries of listeners that bound. |
| `set_listener_enabled(id, enabled)` | Toggles one listener without restarting the others. |
| `listener_status()` | Config, running flag and last bind error per listener. |
| `list_host_ips()` | Local interface addresses, for binding to a LAN IP. |
| `get_history_settings()` / `set_history_settings(s)` | History row limiter. Enabling it prunes immediately. |

## Certificate

| Method | Notes |
| --- | --- |
| `ca_info()` | Common name, organization, validity, fingerprint, and file paths. |
| `ca_pem()` / `export_ca(path, CaFormat::{Pem, Der})` | Export the root CA. |
| `regenerate_ca()` | New root CA. Clients must trust it again. |

## Intercept

| Method | Notes |
| --- | --- |
| `intercept_config()` | `(enabled, mode, source_scope)`, as saved. |
| `toggle_interceptor(enabled, mode)` | Mode is `request`, `response` or `both`. |
| `set_intercept_source_scope(scope)` | Limit to some listeners (`SourceScope { all, ids }`). |
| `get_intercept_rules()` / `update_intercept_rules(rules)` | Whitelist (`intercept`) or blacklist (`pass`) rules. |
| `forward_intercepted_flow(id, url, method, headers, body)` | Forward, optionally modified. `None` fields keep the original. |
| `drop_intercepted_flow(id)` | Abort the flow. |
| `forward_all_intercepted()` / `drop_all_intercepted()` | Returns how many flows were released. |

## Rewrite

| Method | Notes |
| --- | --- |
| `rewrite_rules()` / `save_rewrite_rules(rules)` | Saving also stores the order. |
| `rewrite_enabled()` / `set_rewrite_enabled(b)` | Global switch. |
| `rewrite_source_scope()` / `set_rewrite_source_scope(scope)` | Listener scope. |
| `rewrite_history(limit)` / `clear_rewrite_history()` | Hit log. |
| `proxy::rewrite::preview_rewrite(rule, sample)` | Applies one rule to sample text, for the editor preview. |

## Fuzzer

| Method | Notes |
| --- | --- |
| `start_fuzz(FuzzConfig)` | Runs in the background and discards the previous unsaved run. Returns `FuzzStart { run_id, names, total }`. |
| `stop_fuzz()` | Drops in-flight requests. |
| `fuzz_rows(run_id)` / `fuzz_result(run_id, idx)` | Light rows for the table; the full request and response for one row. |
| `search_fuzz_responses(run_id, query)` | Row indices whose response contains the query. |
| `save_fuzz_run(id, name)` / `is_fuzz_run_saved(id)` / `list_fuzz_runs()` / `load_fuzz_run(id)` / `delete_fuzz_run(id)` / `clear_fuzz_runs()` | Saved runs. |

## WebSockets

| Method | Notes |
| --- | --- |
| `ws_mitm_enabled()` / `set_ws_mitm_enabled(b)` | Whether proxied WebSockets are intercepted. |
| `connect_ws_client(url, headers)` | Opens a client connection from the app. |
| `send_ws_frame(connection_id, msg_type, payload, to_server)` / `disconnect_ws(connection_id)` | Composer actions. |
| `delete_ws_connection(id)` / `clear_ws_session(id)` / `clear_ws_messages()` | Storage cleanup. |
| `db::ws_db::load_ws_connections` / `load_ws_messages` | Read sessions and frames. |

## Webhooks

| Method | Notes |
| --- | --- |
| `webhook_status()` | `{ port, is_running }`. |
| `start_webhook_receiver(port)` / `stop_webhook_receiver()` | Restarting on the same port retries briefly while the old socket closes. |
| `set_webhook_port(port)` | Saved for the next start. Refused while running. |
| `webhook_endpoints()` / `create_webhook_endpoint(name, path, secret)` / `delete_webhook_endpoint(id)` | Endpoints. |
| `webhook_deliveries(limit)` / `clear_webhook_deliveries()` | Stored deliveries. |
| `verify_webhook(delivery)` | Re-checks against the endpoint's current secret. Returns `(status, expected, provided)`. |
| `calculate_webhook_signature(secret, body, provider)` | GitHub, Stripe, Shopify or plain SHA hex. Also `webhook::calculate_signature`. |
| `replay_webhook(delivery, url)` | Re-sends the delivery. Returns status, body and duration. |

## Workspaces and environments

| Method | Notes |
| --- | --- |
| `get_workspaces()` / `get_workspace(id)` / `get_active_workspace()` / `set_active_workspace(id)` | The active workspace is saved. |
| `create_workspace` / `rename_workspace` / `update_workspace_description` / `delete_workspace` | |
| `get_workspace_environments(ws)` / `get_workspace_environment(id)` / `save_workspace_environment(env)` / `delete_workspace_environment(id)` / `duplicate_workspace_environment(id, name)` | Environments hold variables with variants. |
| `set_active_environment(ws, env)` / `get_active_environment()` | |
| `resolve_vars(text)` / `variable_value(name)` | `{{var}}` substitution against the active environment. Returns the unresolved names too. |
| `workspace::import::import_workspace_json_db` / `workspace::export::export_workspace_file_db` | Workspace files (`mitm-workspace` v1, or Postman v2.1 on import). `workspace::file` holds the format types. See [WORKSPACE_FILE.md](WORKSPACE_FILE.md). |

Each method also exists as a `workspace::*_db(db_path, …)` function. The desktop uses those directly.

## Requests, history and data

| Function | Notes |
| --- | --- |
| `db::get_history_page` / `db::get_history_detail` / `db::delete_history_entry` / `clear_all_traffic()` | Traffic history. Bodies load per flow, never in bulk. |
| `repeater::execute::execute_repeater_tab` | Sends a Repeater request and stores its run. |
| `collections::execute::execute_collection_request_db` | Sends a collection request with its pre/post requests and extract rules. |
| `http::steps::*` | Pre/post request steps shared by Repeater and Collections. |
| `http::curl::parse_curl` | cURL command to request. |
| `data_counts()` / `clear_data_category(category)` | Row counts and per-category cleanup for Settings ▸ Data. |
| `list_db_tables()` / `query_db_table(…)` / `get_db_row(table, rowid)` / `run_db_query(sql)` | Database browser. |

## Utilities

Pure functions under `utils`, with no state or IO.

| Module | Functions |
| --- | --- |
| `utils::encoding` | Base64 (std, url-safe), URL, HTML entities, hex (several byte formats), unicode escape, `jwt_decode`. |
| `utils::hashing` | `md5_hex`, `sha1_hex`, `sha256_hex`, `sha512_hex`, `hmac_hex`, `bcrypt_hash`, `bcrypt_verify`. |
| `utils::json` | `format(indent, sort_keys)`, `minify`, `escape`, `canonicalize`. |
| `utils::cvss` | `CvssV3Base` and `CvssV4Base`: `calculate`, `to_vector`, `from_vector`. |
