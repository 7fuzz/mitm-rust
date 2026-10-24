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
| `ca_info` | | `{ pem, fingerprint, common_name, not_before, not_after, scope, domains }`. `scope` is `global` or `domains`; `domains` lists the permitted names of a domain-locked CA. |
| `ca_pem` | | `{ pem }` |
| `subscribe` | | `{}`. From then on the server also pushes event lines on this connection, between replies. |
| `delete_flows` | `{ id, run_id? }` | `{ deleted }`. All of the container's flows, or only one run's. |

`id` is always the container id the browser sends as the proxy username; flows are stored and shown under the container's registered name.

## Events

Pushed to connections that sent `subscribe`: `{"event": "ca_rotated", "data": {...}}`.

| Event | Data |
| --- | --- |
| `ca_rotated` | Whatever `ca_info` returns for the new CA, plus `old_fingerprint`. Sent whenever the root CA is replaced: manual regenerate, a scope change, or auto-rotate. mitm never installs the CA into a trust store itself; the subscriber imports the new one and removes the old. |

Errors: unknown method, missing or empty params.
