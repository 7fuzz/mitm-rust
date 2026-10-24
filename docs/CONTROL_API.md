# Control API

A unix socket the Firefork browser fork uses to register containers, clear their flows and mark runs. Unix only.

- **Path:** `$XDG_RUNTIME_DIR/mitm/control.sock`, or `~/.local/share/mitm/control.sock` without a runtime dir. The directory is `0700` and the socket `0600`; that permission is the authentication.
- **Framing:** one JSON object per line, in both directions. The server answers every request on the same connection, in order.
- **Started by:** `MitmCore::start_control_server()`; the desktop app starts it at launch.

## Messages

Request: `{"id": 1, "method": "start_run", "params": {"id": "7"}}`. `params` may be left out.

Reply: `{"id": 1, "result": {...}}` or `{"id": 1, "error": "message"}`. `id` is echoed back as sent.

A line that isn't valid JSON gets `{"id": null, "error": "..."}` and the connection stays open.

## Methods

| Method | Params | Result |
| --- | --- | --- |
| `status` | | `{ version, token, listeners: [{ label, address, running }] }`. `token` is the shared proxy password. |
| `register_container` | `{ id, name }` | `{}`. Registers or renames; stored flows move to the new name. |
| `remove_container` | `{ id }` | `{}`. Keeps the container's stored flows. |
| `list_containers` | | `[{ id, name, run_id, flows }]` |
| `start_run` | `{ id }` | `{ run_id }`. Flows captured afterwards carry the new run id. Registers an unknown id under its own id as the name. |
| `delete_flows` | `{ id, run_id? }` | `{ deleted }`. All of the container's flows, or only one run's. |

`id` is always the container id the browser sends as the proxy username; flows are stored and shown under the container's registered name.

Errors: unknown method, missing or empty params.
