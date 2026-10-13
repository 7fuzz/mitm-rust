# Workspace files

A workspace file holds one workspace's **environments** and **collections**: everything needed to share a set of requests or keep them in git. Export writes one, and import reads one back, either as a new workspace or into the current one.

Workspace files are JSON with the extension `.mitm.json`. A complete example is in [`examples/online-store.mitm.json`](../examples/online-store.mitm.json).

Not included: Repeater tabs, intercept and rewrite rules, history, and fuzzer runs.

## Minimal file

```json
{
  "format": "mitm-workspace",
  "version": 1,
  "workspace": { "name": "Online Store" },
  "environments": [
    {
      "name": "Local",
      "active": true,
      "variables": [{ "key": "baseUrl", "value": "http://localhost:3000" }]
    }
  ],
  "collections": [
    {
      "name": "Catalog",
      "requests": [{ "name": "List products", "url": "{{baseUrl}}/products" }]
    }
  ]
}
```

## Rules for the whole file

- **`format` and `version` are required.** `format` is always `"mitm-workspace"`. This document describes version `1`. The app refuses files with a newer version instead of guessing.
- **Unknown fields are errors.** A typo like `"header"` instead of `"headers"` is reported, not silently dropped.
- **Defaults can be left out.** Export leaves them out too: `enabled` is `true`, `method` is `GET`, and empty lists and text are omitted.
- **Export is deterministic.** Exporting the same workspace twice gives the same file, with no database ids or timestamps, so it diffs cleanly in git.
- **`$schema` is allowed** at the top level, so editors can validate the file against [`schemas/mitm-workspace-v1.schema.json`](../schemas/mitm-workspace-v1.schema.json).

## `workspace`

| Field | Type | Notes |
| --- | --- | --- |
| `name` | text | Required. Used as the new workspace's name unless you type another one when importing. |
| `description` | text | Optional. |

## `environments`

| Field | Type | Notes |
| --- | --- | --- |
| `name` | text | Required. |
| `active` | bool | The environment in use. Only the first one marked active counts. |
| `variables` | list | See below. |

Importing as a new workspace makes the `active` environment active. Importing into an existing workspace keeps the environment it already uses, unless it has none.

### Variables

| Field | Type | Notes |
| --- | --- | --- |
| `key` | text | Required. Used in requests as `{{key}}`. |
| `value` | text | Becomes the `default` variant. |
| `variants` | list of `{ "name", "value" }` | More named values, e.g. one per test account. |
| `activeVariant` | text | The variant in use, by name. It must name one of the variable's variants. |
| `secret` | bool | Masked in the app. Export leaves the value blank unless you choose **Include secret values**. |
| `enabled` | bool | Defaults to `true`. |

In the app, every variable also has an **`(auto)`** variant. Extract rules write into it. Its values come from runs, so export leaves them out unless you choose **Include (auto) values**.

How a variable is imported:

| In the file | In the app |
| --- | --- |
| only `value` | `(auto)` (empty), `default` = value, **`default` active** |
| `value` and `variants` | `(auto)`, `default` = value, then the variants |
| only `variants` | `(auto)`, then the variants |
| neither | `(auto)` only, empty and active, ready for an extract rule to fill |

- **Which variant is active:** the one `activeVariant` names. Without it, the first imported variant is active (`default` when there's a `value`), or `(auto)` when there's nothing else.
- **`(auto)` starts empty**, unless `variants` lists one named `(auto)`; then its value is used.
- **A variant already named `default`:** the `value` goes to `default1`, or `default2` if that's taken too.

A variant whose value is exactly a credential ref, like `{{$cred:DG/dgAdmin.password}}`, is a link to that credential field. Export always keeps links, even on secret variables and `(auto)` variants, since they hold no secret themselves.

Export reverses this. A variable without variants is written as `value`. A variable with variants is written as its `variants` list, plus `activeVariant` when the active one isn't the first. `(auto)` is only in the list when it has a value and **Include (auto) values** is on. If `(auto)` was active, `activeVariant` still says so.

## `vaults`

The vaults linked to the workspace, with their credential entries but no values. Importing them lets linked variables and `{{$cred:vault/entry.field}}` refs resolve once the values are filled in.

| Field | Type | Notes |
| --- | --- | --- |
| `name` | text | Required. Letters, digits, `-` and `_`. |
| `credentials` | list | Entries: `{ "name", "fields" }`. |

An entry's `name` follows the same rule. Its `fields` are `{ "key", "kind", "extracted" }`: `kind` is `text` (default), `secret` or `totp`, and `extracted` marks a field an extract rule writes (a token).

Files made before vaults have a top-level `credentials` list of entries instead. Import puts those in a vault named after the workspace and adds that vault to the file's `{{$cred:entry.field}}` refs.

## `credentialsSealed`

Present only when the export included credential values. It holds every entry's values and notes, tagged with their vault, encrypted with AES-256-GCM under a key derived from the export passphrase with argon2id. `salt`, `nonce` and `ciphertext` are base64.

## `collections`

Collections are folders, and they nest.

| Field | Type | Notes |
| --- | --- | --- |
| `name` | text | Required. |
| `description` | text | Markdown docs shown in the app. |
| `requests` | list | In display order. |
| `collections` | list | Subfolders. |

### Requests

| Field | Type | Notes |
| --- | --- | --- |
| `name` | text | Required. |
| `url` | text | Required. The full URL, usually starting with a variable: `{{baseUrl}}/orders`. |
| `method` | text | Defaults to `GET`. |
| `description` | text | Markdown docs. |
| `params` | list of name/value rows | Query parameters. |
| `headers` | list of name/value rows | |
| `body` | object | See below. Leave it out for no body. |
| `extract` | list | Rules that save part of the response into a variable. |
| `ref` | text | A name for this request that `before` and `after` can point at. Unique in the file. |
| `before` | list of refs | Requests to run first, in order (pre-requests). |
| `after` | list of refs | Requests to run afterwards (post-requests). |

**Name/value rows** are used by `params`, `headers` and urlencoded bodies: `{ "name": "page", "value": "1" }`. Add `"enabled": false` to keep a row but not send it. Rows keep their order, and the same name can appear twice.

**`ref`** only needs to be set on requests that others run. Export generates one from the name (`Log in` → `log-in`, then `log-in-2`, …). A `before` or `after` that doesn't match a `ref` is an error.

### `body`

The `type` picks the shape:

```json
{ "type": "json", "content": "{\n  \"productId\": 42\n}" }
{ "type": "raw", "content": "plain text" }
{ "type": "urlencoded", "fields": [{ "name": "email", "value": "{{customerEmail}}" }] }
{ "type": "form", "fields": [
    { "name": "alt", "value": "Red running shoe" },
    { "name": "image", "type": "file", "value": "/path/to/shoe.png" }
] }
```

- **`json` and `raw`** keep `content` exactly as written, including formatting and `{{variables}}`.
- **`form` fields** take a `type`:
  - `text`: the default.
  - `file`: `value` is a file path, read when the request is sent.
  - `base64`: `value` is the file's bytes.

  `fileName` and `contentType` are optional.

### `extract`

```json
{ "variable": "accessToken", "from": "json", "expression": "data.token" }
```

| `from` | `expression` |
| --- | --- |
| `json` | A JSON path into the response body, e.g. `data.token` |
| `header` | A response header name |
| `regex` | A regular expression; the first capture group is saved, or the whole match without one |
| `after` | Text after a marker, as `marker\|\|length` (length is optional, default 256) |
| `before` | Text before a marker, as `marker\|\|length` (length is optional, default 256) |
| `between` | Text between two markers, as `start\|\|end` |

The value goes into the variable's `(auto)` variant in the active environment.

## Importing

Use **⇩ Import…** in the top bar's workspace menu, or **Import workspace…** in the command palette. You can drop a file on the window, choose a file, or paste JSON. The dialog shows what will be created before anything is written, and you pick:

- **New workspace:** named after `workspace.name`, unless you type a name.
- **Into the current workspace:** its environments and collections are added next to the existing ones.

Vaults are matched by name. One that already exists is linked to the workspace rather than copied, and an entry already in it keeps its fields and gains the ones it lacks. A vault that doesn't exist is created and linked. When the file has `credentialsSealed`, the dialog asks for the export passphrase. Skipping it imports names and field keys with empty values.

An import either fully succeeds or changes nothing.

The same dialog also reads **Postman v2.1 collections**: folders, requests, headers, raw bodies, and collection variables (as one environment).

## Exporting

Use **⇧ Export…** in the workspace menu, or **Export workspace…** in the command palette. Unsaved edits are saved first, so the file matches what's on screen.

If there's something to decide, a prompt asks first. Both options are off by default:

- **Include secret values**: shown when the workspace has secret variables. Without it, their values are blank.
- **Include (auto) values**: shown when some `(auto)` variant has a value. Without it, `(auto)` variants are left out. It also covers credential fields written by extract rules.
- **Include credentials**: shown when the workspace's vaults have credentials. Asks for a passphrase and writes the values into `credentialsSealed`. Without it, only names and field keys are exported.
