# Import Project from JSON Specification

This document provides a technical specification and operational guide for the **Import Project from JSON** functionality in MITM Rust. The backend logic is implemented in `src-tauri/src/workspace/import.rs`, `src-tauri/src/workspace/mod.rs`, `src-tauri/src/collections/execute.rs`, and `src-tauri/src/request_steps.rs`.

---

## Architecture Overview

The import system allows importing complete projects—including environments, variables, repeater request collections (groups), and endpoint targets—from a single structured JSON file into the local SQLite database.

```mermaid
flowchart TD
    A[JSON Project File] -->|File API / JS Upload| B[Frontend: useWorkspaceStore / ProjectImportModal]
    B -->|Tauri IPC invoke: import_workspace_json| C[Rust Backend: workspace/import.rs]
    C -->|Selective Filtering| D[Rust Processing Engine]
    D -->|Step 1: Insert Envs & Variables| E[(SQLite: environments)]
    D -->|Step 2: Insert Collection Folders| F[(SQLite: collections)]
    D -->|Step 3: Insert Requests & Extract Rules| G[(SQLite: requests)]
    G -->|Step 4: Resolve pre/post request names| I[(SQLite: request_steps)]
    D -->|Step 5: PRAGMA optimize| H[Return Import Summary JSON]
```

---

## Dynamic & Built-in Variables (Postman Compatible)

MITM Rust includes native support for dynamic, runtime-evaluated template variables in URLs, Request Headers, Query Parameters, and Bodies. Dynamic variables are resolved on the fly during request execution and cURL / Wire-format previews:

### Supported Dynamic Variables

| Variable | Description | Example Output |
| :--- | :--- | :--- |
| `{{$timestamp}}` | Current Unix timestamp in seconds | `1788073927` |
| `{{$timestampMs}}` / `{{$timestamp_ms}}` | Current Unix timestamp in milliseconds | `1788073927580` |
| `{{$isoTimestamp}}` / `{{$iso_timestamp}}` | ISO-8601 UTC timestamp string | `2026-08-30T07:44:23Z` |
| `{{$currentDate}}` / `{{$date}}` | Current UTC date (`YYYY-MM-DD`) | `2026-08-30` |
| `{{$currentTime}}` / `{{$time}}` | Current UTC time (`HH:MM:SS`) | `07:44:23` |
| `{{$uuid}}` / `{{$guid}}` / `{{$randomUUID}}` | Random UUID version 4 (default) | `278d9778-cce7-4d70-938f-65746bda14bc` |
| `{{$uuidv7}}` / `{{$randomUUIDv7}}` / `{{$guidv7}}` | Time-ordered UUID version 7 | `0191a3c6-6d55-7b3e-967a-1123456789ab` |
| `{{$randomInt}}` / `{{$random_int}}` | Random integer between `1` and `1000` | `742` |
| `{{$randomDigit}}` / `{{$random_digit}}` | Random single digit between `0` and `9` | `8` |
| `{{$randomAlphaNumeric}}` | 8-character random alphanumeric string | `a7b9c2d1` |
| `{{$randomHex}}` | 16-character random hexadecimal string | `e4d909c290d0fb1e` |
| `{{$randomEmail}}` | Random fake email address | `user_f685c58f@example.com` |
| `{{$randomPhoneNumber}}` / `{{$randomPhone}}` | Random Indonesian phone number (`+6281...`) | `+628189472619` |
| `{{$randomUserName}}` | Random username string | `user_a1b2c3` |
| `{{$randomCity}}` | Random city name | `Jakarta`, `Bandung`, `Surabaya` |
| `{{$randomCountry}}` | Random country name | `Indonesia`, `Singapore`, `Japan` |
| `{{$randomCountryCode}}` | Random ISO-2 country code | `ID`, `SG`, `US`, `JP` |
| `{{$randomPrice}}` | Random monetary price | `49.99` |
| `{{$randomIPv4}}` / `{{$randomIP}}` | Random IPv4 address | `192.168.1.42` |
| `{{$randomBoolean}}` / `{{$randomBool}}` | Random boolean string | `true` or `false` |

---

## Recursive Variable Interpolation Engine

Variable interpolation in MITM Rust is multi-pass and recursive (supporting up to 15 nested resolution cycles):

1. **Nested Variable Resolution**:
   - If `var_a = "api.example.com"`
   - And `var_b = "https://{{var_a}}/v1"`
   - And `endpoint = "{{var_b}}/users"`
   - Resolving `{{endpoint}}` automatically expands to `https://api.example.com/v1/users`.
2. **Whitespace Tolerance**:
   - Both `{{var_name}}` and `{{ var_name }}` (with arbitrary spacing) are recognized and resolved identically.
3. **Case-Insensitive Fallback**:
   - If exact key matching fails, the engine falls back to case-insensitive key lookup.
4. **Cycle & Infinite Loop Safety**:
   - Guarded against circular references (e.g. `a = "{{b}}"`, `b = "{{a}}"`) without stack overflows or crashes.

---

## Best Practices: URL & Host Variable Templating (`{{url}}` / `{{host}}`)

To ensure clean separation between request definitions and environment configurations, you can use either or both of the following flexible strategies depending on your testing needs:

### Strategy 1: Full Base URL (`{{url}}`)
Store the complete URL including protocol directly in the `url` environment variable:
- **Variable `url`**: `https://api.example.com` or `http://localhost:8080`
- **Request Format**: `"url": "{{url}}"` or `"url": "{{url}}/v1"`

### Strategy 2: Host (Domain / IP:Port) Composition (`http://{{host}}`)
Store the domain, IP, or `host:port` in `host` (e.g. `example.com` or `localhost:8080`), and compose the protocol explicitly in the request or variable definition:
- **Variable `host`**: `api.example.com` or `localhost:8080`
- **Request Format**: `"url": "http://{{host}}"` or `"url": "https://{{host}}"`

### Strategy 3: Dynamic Dual / Composed Variables
You can also define `url` dynamically as `http://{{host}}` or `https://{{host}}` in variable configurations so that requests referencing either `{{url}}` or `{{host}}` resolve seamlessly!

### Pattern Comparison:
- **Full URL Variable**: `"url": "{{url}}"` (where `url` = `https://api.example.com` or `http://localhost:8080`)
- **Explicit Protocol + Host**: `"url": "http://{{host}}"` (where `host` = `localhost:8080`)
- **Composed Protocol + Host**: `"url": "{{protocol}}://{{host}}/v1"` (where `protocol` = `http` and `host` = `localhost:8080`)

---

## Rust Struct & Data Contracts

The Rust backend in `src-tauri/src/repeater.rs` defines the payload contract using `serde::Deserialize`:

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct ImportRepeaterData {
    pub name: Option<String>,
    pub url: Option<String>,
    pub header: Option<serde_json::Value>,
    pub placeholders: Option<serde_json::Value>,
    pub all_environments: Option<Vec<serde_json::Value>>,
    pub all_variables: Option<Vec<serde_json::Value>>,
    pub test_cases: Option<Vec<serde_json::Value>>,
    pub import_environments: Option<Vec<String>>,
    pub import_groups: Option<Vec<String>>,
    pub link_to_environment: Option<String>,
    pub link_to_environments: Option<Vec<String>>,
    pub smart_link: Option<bool>,
}
```

---

## JSON Format Specification

### Root Schema

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `name` | String | No | Project display name |
| `url` | String | No | Global base URL fallback (Best practice: `"https://{{url}}"` or `"https://{{host}}"`) |
| `header` | Object | No | Key-value dictionary of global HTTP headers |
| `placeholders` | Object | No | Key-value dictionary for simple variable fallbacks |
| `all_environments` | Array | No | List of environment definitions |
| `all_variables` | Array | No | List of environment variable definitions |
| `test_cases` | Array | No | List of repeater groups (collections) and request targets |
| `import_environments` | Array | No | Array of environment IDs or names selected for import |
| `import_groups` | Array | No | Array of group names selected for import |
| `link_to_environment` | String | No | Single target environment ID to link imported groups to (legacy) |
| `link_to_environments` | Array | No | Target environment IDs to link imported groups to |
| `smart_link` | Boolean | No | If `true`, automatically links imported groups to newly imported environments |

---

### Nested Schema Definitions

#### 1. `all_environments` Item
```json
{
  "id": "env_dev_001",
  "name": "Development"
}
```

#### 2. `all_variables` Item (Host & Token Variables)
```json
{
  "environmentId": "env_dev_001",
  "name": "host",
  "activeIndex": 0,
  "values": [
    {
      "name": "Dev Server",
      "value": "https://dev-api.example.com"
    },
    {
      "name": "Localhost",
      "value": "http://localhost:8080"
    }
  ]
}
```

#### 3. `test_cases` Item (Repeater Group & Multi-Level Nested `folders`)
```json
{
  "name": "Authentication API",
  "url": "{{host}}/v1",
  "description": "# Authentication API Collection\n\nThis collection contains user authentication endpoints and nested subfolders.",
  "target": [
    {
      "name": "Root Login Request",
      "method": "POST",
      "endpoint": "/auth/login",
      "header": {
        "Content-Type": "application/json"
      },
      "body": "{\"username\":\"admin\",\"password\":\"secret\"}"
    }
  ],
  "folders": [
    {
      "name": "OAuth 2.0 Subfolder",
      "description": "Subfolder for OAuth 2.0 flows",
      "target": [
        {
          "name": "Get Bearer Token",
          "method": "POST",
          "endpoint": "/oauth/token"
        }
      ],
      "folders": []
    }
  ]
}
```

##### Markdown `description` Format Rules:
- Both **Collections** (`test_cases` items) and **Individual Requests** (`target` items) support an optional `description` string.
- The `description` field accepts full **GitHub Flavored Markdown (GFM)**:
  - Headers (`#`, `##`, `###`)
  - Code Blocks (` ```json ... ``` `)
  - Alert Callouts (`> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, `> [!IMPORTANT]`)
  - Tables (`| col | col |`)
  - Lists (`- item`, `1. item`)
  - Inline Code (`` `variable` ``) and Bold/Italic formatting.
- When imported, `description` strings are automatically stored into SQLite (`repeater_groups.description` and `repeater_requests.description`) and rendered natively in the UI using `react-markdown` and `remark-gfm`.

#### 4. Base64 File Payloads (`application/json` & `multipart/form-data`)

MITM Rust supports embedding binary files directly within Repeater request bodies as Base64 strings.

##### A. `application/json` Base64 File Embedding
Files can be attached directly into JSON request payloads as Base64 Data URIs (`data:<mime>;base64,<payload>`). The UI JSON tree editor includes a `FILE` type option that converts local disk files to Base64 strings automatically.

```json
{
  "name": "JSON Base64 File Upload",
  "method": "POST",
  "endpoint": "/v1/documents/upload",
  "header": {
    "Content-Type": "application/json"
  },
  "body": "{\"documentName\":\"invoice.pdf\",\"fileData\":\"data:application/pdf;base64,JVBERi0xLjQK...\"}"
}
```

##### B. `multipart/form-data` Base64 Form Uploads
For multipart form requests, Base64 files use the `__form_data` array structure with `"type": "base64"`. The Rust execution engine (`src-tauri/src/repeater_execute.rs`) decodes the Base64 payload and constructs valid multipart HTTP boundaries automatically upon sending.

```json
{
  "name": "Multipart Base64 Form Upload",
  "method": "POST",
  "endpoint": "/files/upload",
  "header": {
    "Content-Type": "multipart/form-data"
  },
  "body": "{\"__form_data\":[{\"k\":\"file\",\"v\":\"data:application/pdf;base64,JVBERi0xLjQK...\",\"type\":\"base64\",\"fileName\":\"invoice.pdf\",\"contentType\":\"application/pdf\"}]}"
}
```

---

## Backend Processing Engine (`src-tauri/src/repeater.rs`)

### 1. Environments & Variables Ingestion

1. **Selective Filter**: An environment from `all_environments` is processed only if `import_environments` is empty/omitted OR contains the environment's `id` or `name`.
2. **Environment Insertion**:
   - A new UUID is generated for the environment in the database.
   - Inserted into `environments(id, name, is_active)` with `is_active = 0`.
3. **Variable Ingestion**:
   - Matches `all_variables` entries where `environmentId` equals the source environment `id`.
   - Generates new UUID for `variables(id, environment_id, name, active_index)`.
   - Iterates through `values` and inserts into `variable_values(id, variable_id, name, value)`.

### 2. Repeater Group & Request Ingestion

1. **Selective Filter**: A group from `test_cases` is imported only if `import_groups` is empty/omitted OR contains the group's `name`.
2. **Group Insertion**:
   - Generates new UUID for `repeater_groups(id, name, order_index)`.
3. **Environment Linking**:
   - Links group to environment IDs specified in `link_to_environments` / `link_to_environment`.
   - If `smart_link: true`, automatically adds links to newly created environment IDs.
   - Inserts records into `environment_groups(group_id, environment_id)`.
4. **URL Resolution Logic**:
   - Base URL precedence: `group.url` -> `global.url` -> `""`.
   - Concatenates `base_url` + `request.endpoint`.
   - Formats `params` as key-value query parameters (`?key1=val1&key2=val2`) and appends to URL.
   - Preserves `{{host}}` and `{{url}}` variable placeholders for runtime substitution by the active environment context.
5. **Header Resolution Logic**:
   - Starts with global `header` map.
   - Overrides or adds keys from request-level `header`.
   - If request header key maps to `null`, the header is **deleted/removed**.
6. **Request Insertion**:
   - Inserts into `repeater_requests(id, name, group_id, method, url, headers, body, extract, order_index)`.
7. **Pre/Post-Request Linking**:
   - Runs after every request in the file has been inserted, so `pre_requests` / `post_requests` may name requests that appear later in the file.
   - Each name is matched against the names of requests from **this import only**. A request in the same folder wins, otherwise the first match in file order is used.
   - Matches are stored in order in the `request_steps` table. Unknown names are ignored, and a request can't be its own step.

---

## Returned Response Contract

On success, `import_repeater_data` returns:

```json
{
  "success": true,
  "imported": {
    "environments": 2,
    "variables": 5,
    "groups": 3,
    "requests": 12
  }
}
```

---

## Complete Example JSON File (Best Practice Template)

Below is a production-grade sample project JSON file demonstrating best practices with `https://{{host}}` and `https://{{url}}` variable templating:

```json
{
  "name": "E-Commerce API Suite",
  "url": "https://{{host}}",
  "header": {
    "Accept": "application/json",
    "X-Client-ID": "mitm-desktop-app"
  },
  "all_environments": [
    {
      "id": "env_dev_001",
      "name": "Development"
    },
    {
      "id": "env_prod_001",
      "name": "Production"
    }
  ],
  "all_variables": [
    {
      "environmentId": "env_dev_001",
      "name": "host",
      "activeIndex": 0,
      "values": [
        {
          "name": "Dev Server",
          "value": "https://dev.api.store.com"
        },
        {
          "name": "Localhost",
          "value": "http://localhost:3000"
        }
      ]
    },
    {
      "environmentId": "env_dev_001",
      "name": "token",
      "activeIndex": 0,
      "values": [
        {
          "name": "Dev Token",
          "value": "dev_bearer_token_123"
        }
      ]
    },
    {
      "environmentId": "env_prod_001",
      "name": "host",
      "activeIndex": 0,
      "values": [
        {
          "name": "Production Gateway",
          "value": "https://api.store.com"
        }
      ]
    },
    {
      "environmentId": "env_prod_001",
      "name": "token",
      "activeIndex": 0,
      "values": [
        {
          "name": "Prod Token",
          "value": "prod_bearer_token_999"
        }
      ]
    }
  ],
  "test_cases": [
    {
      "name": "User Service",
      "url": "https://{{host}}/v1",
      "target": [
        {
          "name": "Get Profile",
          "method": "GET",
          "endpoint": "/profile",
          "params": {
            "include_meta": "true"
          },
          "header": {
            "Authorization": "Bearer {{token}}"
          },
          "body": "",
          "extract": [
            {
              "type": "json",
              "targetVariable": "userId",
              "expression": "user.id"
            },
            {
              "type": "after_string",
              "targetVariable": "sessionToken",
              "expression": "token=||64"
            },
            {
              "type": "between_string",
              "targetVariable": "csrfToken",
              "expression": "csrf_token=\"||\""
            },
            {
              "type": "header",
              "targetVariable": "AUTH_HEADER",
              "expression": "Authorization"
            },
            {
              "type": "body_regex",
              "targetVariable": "REG_ID",
              "expression": "id=([0-9]+)"
            }
          ]
        }
      ]
    },
    {
      "name": "Payment Gateway",
      "url": "https://{{url}}",
      "target": [
        {
          "name": "Process Payment",
          "method": "POST",
          "endpoint": "/checkout/pay",
          "header": {
            "Authorization": "Bearer {{token}}",
            "Content-Type": "application/json"
          },
          "body": "{\"amount\": 99.99, \"currency\": \"USD\"}",
          "extract": {
            "transactionId": "data.txn_id"
          }
        }
      ]
    }
  ]
}
```

---

### Auto-Extraction Specification (`extract` / `extract_rules`)

The `extract` (or `extract_rules`) field on requests supports both **Shorthand Dictionary Format** and **Advanced Multi-Mode Array Format**.

When a request with extract rules completes execution, the extracted value is automatically assigned to the **`(auto)`** variant of the target environment variable in the active environment.

#### 1. Shorthand Dictionary Format (Legacy / JSON Mode)
Maps target variable names to JSON path expressions:
```json
"extract": {
  "userId": "user.id",
  "authToken": "data.token",
  "firstItemId": "data.0.id"
}
```

#### 2. Advanced Multi-Mode Array Format
Supports 6 distinct extraction modes with JSONPath array index support and case-insensitivity:

| Mode (`type`) | Description | Expression Format (`expression`) | Example |
| :--- | :--- | :--- | :--- |
| `json` / `jsonpath` | JSON Path / Dot notation / Array Indexing | Dot path or bracket index | `"data.0.id"`, `"$.data[0].id"`, `"data.access_token"` |
| `after_string` | Extract text after prefix string | `prefix\|\|max_chars` | `"token=\|\|64"` |
| `before_string` | Extract text before suffix string | `suffix\|\|max_chars` | `"&expires=\|\|64"` |
| `between_string` | Extract text between start & end delimiters | `start_delim\|\|end_delim` | `"session=\"\|\|\""` |
| `header` | Extract from HTTP response header | Header name | `"Authorization"`, `"Set-Cookie"` |
| `body_regex` / `regex` | Extract using Regex capture group | Regex pattern | `"id=([0-9]+)"` |

##### Multi-Mode Array Example:
```json
"extract": [
  {
    "type": "json",
    "targetVariable": "account_id",
    "expression": "data.0.id",
    "enabled": true
  },
  {
    "type": "json",
    "targetVariable": "access_token",
    "expression": "data.access_token",
    "enabled": true
  },
  {
    "type": "after_string",
    "targetVariable": "ACCESS_TOKEN",
    "expression": "token=||128",
    "enabled": true
  },
  {
    "type": "between_string",
    "targetVariable": "CSRF_TOKEN",
    "expression": "csrf_token=\"||\"",
    "enabled": true
  },
  {
    "type": "header",
    "targetVariable": "SESSION_HEADER",
    "expression": "Set-Cookie",
    "enabled": true
  }
]
```

##### Automatic `(auto)` Variant Population:
- When a response is received, the extraction engine evaluates all enabled rules.
- The extracted string is assigned to variant `0` (`name: "(auto)"`) of the target variable in the active environment.
- If the variable currently has `(auto)` selected (`activeIndex: 0`), the variable's active `value` is also updated immediately.
- The frontend store automatically syncs the newly extracted values across all views without requiring a manual refresh.

---

### Pre- and Post-Requests: Fetch a CSRF / XSRF Token Before Sending (`pre_requests` / `post_requests`)

Similar to session-handling macros in Burp Suite, a request can list other requests to run **before** and **after** it every time it is sent. This keeps one-time tokens such as CSRF/XSRF tokens fresh:

1. The pre-requests run in order, and their `extract` rules save values into the active environment (e.g. `csrf_token`).
2. The main request is then interpolated, so `{{csrf_token}}` resolves to the token that was just fetched.
3. The post-requests run in order (e.g. a logout or cleanup call). Values extracted from the main response are already available to them.

| Field | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `pre_requests` | Array of strings | Names of requests in the same import to run first, in order | `["Login", "Get CSRF Token"]` |
| `post_requests` | Array of strings | Names of requests in the same import to run afterwards, in order | `["Logout"]` |

```json
{
  "name": "Account",
  "url": "https://{{host}}",
  "target": [
    {
      "name": "Login",
      "method": "POST",
      "endpoint": "/login",
      "body_mode": "urlencoded",
      "body_urlencoded": "username={{username}}&password={{password}}"
    },
    {
      "name": "Get CSRF Token",
      "method": "GET",
      "endpoint": "/account/settings",
      "extract": [
        {
          "type": "between_string",
          "targetVariable": "csrf_token",
          "expression": "name=\"csrf\" value=\"||\""
        }
      ]
    },
    {
      "name": "Logout",
      "method": "POST",
      "endpoint": "/logout"
    },
    {
      "name": "Change Email",
      "method": "POST",
      "endpoint": "/account/email",
      "pre_requests": ["Login", "Get CSRF Token"],
      "post_requests": ["Logout"],
      "header": {
        "X-CSRF-Token": "{{csrf_token}}",
        "Content-Type": "application/x-www-form-urlencoded"
      },
      "body_mode": "urlencoded",
      "body_urlencoded": "email=new@example.com&csrf={{csrf_token}}"
    }
  ]
}
```

#### Execution Behavior
- **Flat lists**: only the main request's own lists run. Pre/post-requests configured on a step are ignored while it runs as a step, so add every request the flow needs to the main request's lists.
- **Shared cookies**: every step and the main request share one cookie jar for that send, so a session cookie set by *Login* or alongside the token (e.g. `Set-Cookie: sid=...`) is sent back on the following requests. A request with its own enabled `Cookie` header uses that header instead.
- **History**: each step's run is recorded in that step's own run history. Cookies taken from the jar appear in the logged request headers.
- **Pre-request failures**: if a pre-request can't reach the server (network error, invalid URL), the main request and the post-requests are not sent. Instead, the main request's response and a status `0` entry in its run history read `Pre-request "<name>" failed: <error>`. A pre-request that gets any HTTP response, including 4xx/5xx, or whose extract rule finds nothing, does not stop the send.
- **Post-request failures**: the main response is always what is shown. A failing post-request only appears in its own run history, and the remaining post-requests still run. Post-requests are skipped if the main request itself didn't reach the server.
- **Deleting requests**: steps are stored in the `request_steps` table with cascading foreign keys, so deleting a request (or the folder holding it) removes it from every list it was in.
- **Duplicating**: a duplicated request keeps its steps. When a whole folder is duplicated, steps that point at requests inside that folder point at the copies.
- **Export**: exported projects write the lists back as request names. Give requests unique names if you plan to round-trip them.

> [!NOTE]
> Requests imported via the Postman v2.1 fallback format do not support `pre_requests` / `post_requests`; set them in the request's **Pre/Post** tab after importing.

---

## Multi-Variant Body Storage & Format Modes (`request_bodies` Table)

Starting with Database Migration Version 8, request bodies are stored in a dedicated, non-destructive **`request_bodies`** table. This ensures that switching between `raw`, `json`, `x-www-form-urlencoded`, and `form-data` modes in the editor does not overwrite or destroy data in other formats.

| Entity | DB Table | Key Columns | Description |
| :--- | :--- | :--- | :--- |
| Environment | `environments` | `id`, `name`, `is_active` | Saved environment definitions |
| Variable | `variables` | `id`, `environment_id`, `name`, `active_index` | Global variable definitions |
| Variable Value | `variable_values` | `id`, `variable_id`, `name`, `value` | Variant values per variable |
| Repeater Group | `repeater_groups` | `id`, `name`, `order_index`, `extract`, `description` | Endpoint collection containers |
| Repeater Request | `repeater_requests` | `id`, `name`, `group_id`, `method`, `url`, `headers`, `body`, `extract`, `description`, `response_duration`, `url_params` | Endpoint request definitions with persisted URL parameter toggles & latency |
| Multi-Variant Bodies | `request_bodies` | `request_id`, `body_mode`, `body_raw`, `body_json`, `body_urlencoded`, `body_multipart` | Dedicated format variant buffers per request |
| Group Environment Link | `environment_groups` | `group_id`, `environment_id` | Collection-to-environment links |

---

### Request Target Body & URL Parameter JSON Fields (Optional Specification)

| Field | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `body` | String | Main / Raw request body content | `"{\"key\": \"value\"}"` or `"k1=v1&k2=v2"` |
| `body_mode` | String | Active body mode: `"raw"`, `"json"`, `"urlencoded"`, or `"multipart"` | `"urlencoded"` |
| `body_json` | String | Dedicated JSON body representation | `"{\"amount\": 100}"` |
| `body_urlencoded` | String | Dedicated URL-encoded body string | `"grant_type=client_credentials&client_id={{client_id}}"` |
| `body_multipart` | String | Dedicated Multipart form data JSON structure with parameter toggles | `"{\"__form_data\": [{\"enabled\": true, \"k\": \"file\", \"v\": \"data:...\", \"type\": \"base64\"}]}"` |
| `url_params` | String | JSON string of URL query parameter entries with enable/disable toggles | `"[{\"id\":\"123\",\"k\":\"page\",\"v\":\"1\",\"enabled\":true},{\"id\":\"456\",\"k\":\"filter\",\"v\":\"abc\",\"enabled\":false}]"` |
| `pre_requests` | Array | Names of requests to run first (see [Pre- and Post-Requests](#pre--and-post-requests-fetch-a-csrf--xsrf-token-before-sending-pre_requests--post_requests)) | `["Login", "Get CSRF Token"]` |
| `post_requests` | Array | Names of requests to run afterwards | `["Logout"]` |

#### Parameter & JSON Key ON / OFF Toggles

1. **URL Parameters (`url_params`), Multipart & URL-Encoded Form Data**:
   Each entry includes an optional `"enabled": true | false` property:
   - **`"enabled": true`**: Parameter is active and included when reconstructing URLs or executing HTTP requests.
   - **`"enabled": false`**: Parameter is disabled and excluded from the request URL/payload, but remains persisted in the editor table UI for quick testing.

2. **JSON Object Property Keys (`body_json` / `body`)**:
   - Object keys prefixed with `__disabled_` (e.g. `"__disabled_filter": "xxx"`) are rendered in the visual JSON tree editor as disabled properties (`[ ]` unchecked with strike-through styling).
   - Upon HTTP request execution, the backend automatically strips all `__disabled_` keys recursively so the server receives clean, valid JSON.

---

### Comprehensive Body Variant & Parameter Examples

#### 1. JSON Body with Disabled Property Keys (`body_mode: "json"`)
In `body_json` or `body`, prefixing any object property key with `__disabled_` renders it as an unchecked `[ ]` property in the UI tree editor, and automatically strips it upon sending:

```json
{
  "name": "Create Order with Disabled Filter",
  "method": "POST",
  "endpoint": "/api/v1/orders",
  "body_mode": "json",
  "body_json": "{\n  \"customer_id\": \"cust_123\",\n  \"amount\": 250.00,\n  \"__disabled_discount_code\": \"SUMMER50\",\n  \"items\": [\"item_a\", \"item_b\"]\n}",
  "header": {
    "Content-Type": "application/json"
  }
}
```

#### 2. Structured URL Parameters with Active & Disabled Toggles (`url_params`)
The `url_params` field stores the array of structured URL query parameters, preserving disabled parameters for quick testing without deleting them:

```json
{
  "name": "Search Products",
  "method": "GET",
  "endpoint": "/api/v1/products",
  "url_params": "[\n  {\"id\": \"p1\", \"k\": \"page\", \"v\": \"1\", \"enabled\": true},\n  {\"id\": \"p2\", \"k\": \"limit\", \"v\": \"20\", \"enabled\": true},\n  {\"id\": \"p3\", \"k\": \"category\", \"v\": \"electronics\", \"enabled\": false}\n]",
  "body_mode": "raw"
}
```
*Resulting executed URL:* `/api/v1/products?page=1&limit=20` (`category` is disabled and omitted).

#### 3. URL-Encoded Form Data (`body_mode: "urlencoded"`)
URL-encoded forms support dedicated `body_urlencoded` representation with parameter ON/OFF toggling:

```json
{
  "name": "OAuth 2.0 Client Credentials Token",
  "method": "POST",
  "endpoint": "/oauth/token",
  "body_mode": "urlencoded",
  "body_urlencoded": "grant_type=client_credentials&client_id={{client_id}}&client_secret={{client_secret}}",
  "header": {
    "Content-Type": "application/x-www-form-urlencoded"
  }
}
```

#### 4. Multipart Form Data with File & Disabled Controls (`body_mode: "multipart"`)
Multipart form payloads store `__form_data` arrays supporting text parameters, Base64 files, and `enabled: false` toggles:

```json
{
  "name": "Upload User Profile Avatar",
  "method": "POST",
  "endpoint": "/api/v1/profile/avatar",
  "body_mode": "multipart",
  "body_multipart": "{\n  \"__form_data\": [\n    {\n      \"enabled\": true,\n      \"k\": \"avatar\",\n      \"v\": \"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...\",\n      \"type\": \"base64\",\n      \"fileName\": \"profile.png\",\n      \"contentType\": \"image/png\"\n    },\n    {\n      \"enabled\": true,\n      \"k\": \"userId\",\n      \"v\": \"user_999\",\n      \"type\": \"text\"\n    },\n    {\n      \"enabled\": false,\n      \"k\": \"debug_mode\",\n      \"v\": \"true\",\n      \"type\": \"text\"\n    }\n  ]\n}",
  "header": {
    "Content-Type": "multipart/form-data"
  }
}
```
