# Import Functionality

Selectively import projects with environments, variables, and repeater collections from exported JSON files.

## Quick Start

1. **Open Workspace**: Click "Workspace" tab in the sidebar
2. **Click Import Project**: Select "Import Project" button
3. **Choose File**: Select a JSON export file
4. **Select Items**: Choose which environments and collections to import
5. **Confirm**: Click "Start Import"
6. **Done**: Notification shows summary of imported items

## How It Works

### Flow Diagram
```
User selects file
    ↓
File parsed and displayed in ImportModal
    ↓
User selects which environments/groups to import
    ↓
finalizeImport() transforms selections
    ↓
Invokes Rust command: import_repeater_data
    ↓
Rust backend creates database records:
  - Environments & Variables
  - Repeater Groups & Requests
  - Links between groups and environments (Smart Link)
    ↓
UI refreshes and shows notification
```

### What Gets Imported

#### Environments & Variables
- Creates new environments with custom names
- Imports environment-scoped variables
- Supports multiple values per variable
- Each variable tracks which value is "active"

#### Repeater Collections & Requests
- Creates repeater groups (collections)
- Imports individual HTTP requests with:
  - **URLs**: Resolved from global + group + endpoint
  - **Headers**: Merged from global + request-specific
  - **Parameters**: Query string parameters
  - **Body**: Request body (for POST/PUT)
  - **Extract**: Response parsing patterns

#### Smart Linking
- When enabled: Automatically links imported collections to imported environments
- Allows environment context switching for collections

### URL Templating Best Practices (`{{host}}` / `{{url}}`)

Using variable placeholders such as `https://{{host}}` or `https://{{url}}` for base URLs is recommended. This allows environment switching to dynamically update target domain names without modifying endpoint paths.

### Expected JSON Structure
```json
{
  "name": "Project Name",
  "url": "https://{{host}}",
  "header": {
    "Authorization": "Bearer {{token}}",
    "Accept": "application/json"
  },
  "placeholders": {
    "token": "default_token_value"
  },
  "all_environments": [
    {
      "id": "env_prod",
      "name": "Production"
    }
  ],
  "all_variables": [
    {
      "environmentId": "env_prod",
      "name": "host",
      "activeIndex": 0,
      "values": [
        {
          "name": "Default",
          "value": "api.example.com"
        }
      ]
    },
    {
      "environmentId": "env_prod",
      "name": "token",
      "activeIndex": 0,
      "values": [
        {
          "name": "Default",
          "value": "prod_token_123"
        }
      ]
    }
  ],
  "test_cases": [
    {
      "name": "Authentication",
      "url": "https://{{host}}/v1",
      "target": [
        {
          "name": "Login",
          "method": "POST",
          "endpoint": "/auth/login",
          "header": {
            "Content-Type": "application/json"
          },
          "body": "{\"email\": \"user@example.com\"}",
          "params": {},
          "extract": {}
        }
      ]
    }
  ]
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name (displays in ImportModal) |
| `url` | string | Global fallback URL |
| `header` | object | Global headers (merged into all requests) |
| `placeholders` | object | Legacy format: simple key-value variables |
| `all_environments` | array | Environments to import |
| `all_variables` | array | Variables associated with environments |
| `test_cases` | array | Repeater collections (groups) |

### URL Resolution

URLs are built in this order:
1. Start with global `url`
2. Override with group-level `url` if specified
3. Append `endpoint` from request
4. Add query `params`

**Example**: `https://api.example.com` + `/v1` + `/users` + `?limit=10`
= `https://api.example.com/v1/users?limit=10`

### Header Merging

1. Start with global `header`
2. Extend/override with request-specific `header`
3. Null values **remove** headers
4. Request headers take precedence

**Example**:
```json
{
  "global": { "Authorization": "Bearer token", "Accept": "json" },
  "request": { "Accept": "xml", "Custom": "value" }
}
// Result: { "Authorization": "Bearer token", "Accept": "xml", "Custom": "value" }
```

## Testing

### Sample File
Location: `test_import_sample.json`

Contains:
- **2 Environments**: Development, Production
- **2 Variables**: token (with environment-specific values)
- **2 Collections**: Authentication, Users
- **5 Requests**: Login, Logout, Get User, Create User, Search Users

### Test Steps

1. **Full Import**
   ```
   1. Open Workspace → Import Project
   2. Select test_import_sample.json
   3. Keep all items selected (default)
   4. Click "Start Import"
   5. Verify: Environments tab shows new environments
   6. Verify: Repeater tab shows new collections and requests
   ```

2. **Selective Import**
   ```
   1. Open Workspace → Import Project
   2. Select test_import_sample.json
   3. Uncheck some environments/collections
   4. Click "Start Import"
   5. Verify: Only selected items were created
   ```

3. **Smart Linking**
   ```
   1. Enable "Smart Link" checkbox
   2. Check "Start Import"
   3. Verify: Groups appear in environments
   4. Verify: Can switch environments and see linked groups
   ```

## Implementation Details

### Files Modified

| File | Change |
|------|--------|
| `src-tauri/src/repeater.rs` | Added `ImportRepeaterData` struct and `import_repeater_data` command |
| `src-tauri/src/lib.rs` | Registered `import_repeater_data` in Tauri handler |
| `src/hooks/traffic/useRepeater.ts` | Implemented `finalizeImport` function |

### Database Tables

Import creates records in:
- `environments` - new environments
- `variables` - environment variables
- `variable_values` - variable values
- `repeater_groups` - collection groups
- `repeater_requests` - individual requests
- `environment_groups` - group-to-environment links (if Smart Link enabled)

### Performance

- **Typical import**: <1 second
- **DB optimization**: Automatic `PRAGMA optimize` after import
- **UI refresh**: Immediate via `refreshRepeater()`
- **Threading**: No blocking operations

## Troubleshooting

### Import Modal Doesn't Appear
- Verify JSON file is valid (use a JSON validator)
- Check browser console for errors
- Ensure file contains `test_cases` array

### "Import failed" Error
- Verify JSON schema matches expected format
- Check that `all_environments` or `placeholders` exist
- Ensure database has write permissions
- Check that required fields (name, url) are present

### Items Not Created
- Verify items were selected in ImportModal before importing
- Check "Smart Link" setting matches your intent
- Try re-importing with simpler selections

### Database Not Updated
- Close and reopen the app
- Try importing again
- Check disk space for database

### URL or Headers Look Wrong
- Verify `global url` + `group url` + `endpoint` equals desired URL
- Check header merging: request headers override global
- Null values should remove headers (verify in JSON)

## Export Format

When exporting projects from MITM Real, the file automatically includes:
- Project metadata (name, global URL, headers)
- All environments and variables
- All repeater collections and requests
- Extract patterns

This format is directly compatible with the import functionality.

## Advanced Usage

### Importing Multiple Projects

You can import multiple project files sequentially:
1. Import Project A (full import)
2. Import Project B (selective import)
3. Items from both projects coexist in your workspace

### Merging with Existing Data

Imports always create new items (no overwrite):
- Use selective import to pick which items to import
- Existing items are never modified
- Duplicate names are allowed (system generates unique IDs)

### Legacy Format Support

If importing a project with only `placeholders` (no environments):
- Frontend automatically wraps placeholders in a virtual "Variables (Legacy)" environment
- You can still import as a new environment

## Compilation Status

✅ Rust: `cargo check --lib` passes  
✅ TypeScript: No type errors  
✅ Integration: Command properly registered  

## Notes

- All UUIDs are auto-generated
- Headers can be overridden at request level
- Null values in headers remove those headers
- Query parameters are URL-encoded in the final URL
- Extract patterns stored as JSON
- Variables support multiple values per environment
- Smart Link creates `environment_groups` records for context switching
