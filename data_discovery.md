# UI Specification: Log Discovery & Analytics Dashboard

## 1. Overview
Create a modern, dark/light-theme capable Log Discovery & Analytics dashboard UI inspired by observability tools like Grafana, Datadog, or VictoriaLogs. The layout consists of a left icon sidebar, an expandable fields sidebar, and a main content area containing search controls, a histogram chart, and a detailed data table.

---

## 2. Layout Structure

### A. Global Top Navigation Header
- **Left**:
  - Logo / Product Name (e.g., "LC")
  - Workspace / Team Dropdowns: `Demo Team` v / `WAF Security` v
  - Quick Info Badges: Datasource indicator (`VictoriaLogs`), Time Range (`Last 3h`), Timezone (`Local`), Limit (`100 rows`)
- **Right**:
  - Current Time Display (e.g., `14:26:17`)
  - Fullscreen toggle icon
  - Share icon
  - Settings icon

### B. Leftmost Navigation Rail (Icon Bar)
- Vertical bar containing icons for main app modules:
  - Search / Discovery (Active)
  - Dashboard
  - Alerts / Notifications
  - Metrics / Charts
  - Users / Teams
  - Integrations / API
  - Settings / Admin
  - User Profile Avatar (`DA` at the bottom)

### C. Left Sidebar (Fields Explorer)
- **Header**:
  - Title: "Fields"
  - Search input box: "Search fields..." with a refresh icon
- **Grouped Field Categories** (collapsible lists with icon, name, badge count, and action buttons):
  1. **CORE FIELDS (3)**
     - `_msg` (String) [click]
     - `L` (String) [click]
     - `_time` (DateTime)
  2. **CONTEXT FIELDS (2)**
     - `host` (String) [click]
     - `status` (String) [click]
  3. **FILTERABLE FIELDS (4)**
     - `duration_ms` (Number) (10)
     - `region` (String) [click]
     - `request_id` (String) [click]
     - `service_name` (String) [click]
  4. **SYSTEM FIELDS (2)**
     - `_stream` (String) [click]
     - `_stream_id` (String) [click]
- **Footer Legend**:
  - `+ Click value to add filter`
  - `- Click minus to exclude`

---

## 3. Main Content Area

### 1. Query & Search Bar Section
- **Mode Tabs**: `Search` (Selected), `LogsQL`, `AI Assistant`, `History`
- **Query Input Field**:
  - Monospace text editor styling
  - Current value: `lvl="ERROR"`
- **Right Action Group**:
  - Datasource selector: `VictoriaLogs`
  - `Collections` dropdown
  - `Live` streaming toggle button
  - Refresh Interval dropdown: `30s`
  - Primary Action Button: `Run` (Green background)

### 2. Histogram Section
- **Toolbar**:
  - Collapse/Expand chevron icon
  - Section Title: "Histogram"
  - Summary stats: `100 logs · 2ms`
  - Right controls: `Group By: No Grouping`, Share icon, View options (JSON/Raw/Chart)
- **Chart Component**:
  - Subtitle / Guide: "5m buckets · Hover to inspect · drag to select range · click bar to zoom"
  - Bar Chart: Vertical blue bars representing log frequency over time
  - X-Axis: Timestamps (e.g., `11:33`, `11:50`, `12:06`, `12:23`, etc.)
  - Y-Axis: Log counts (`0`, `2`, `4`, `6`, `8`, `10`)

### 3. Log Results Table Section
- **Toolbar**:
  - Stats: `2ms · 100 rows`
  - Timezone Selector: `Local` | `UTC`
  - Page Size Dropdown: `50`
  - Pagination Controls: `<<` `<` `1/2` `>` `>>`
  - Column Configuration: `Columns (6)` button
  - Table Filter / Search Input: "Search..."
- **Data Table**:
  - **Headers**:
    - `_time` (Sortable)
    - `L` (Level)
    - `_msg` (Message)
    - `duration_ms`
    - `host`
    - `status`
  - **Row Elements & Styling**:
    - Expandable row arrow icons (`>`) on the far left
    - `_time`: Monospace timestamp (e.g., `2026-08-08T14:24:05.726+...`)
    - `L` (Log Level): Highlighting badge. Red background with white text for `ERROR`
    - `_msg`: Truncated log message text (e.g., `geo database looku...`, `upstream origin ti...`, `TLS handshake fail...`)
    - `duration_ms`: Numeric values right/left aligned (e.g., `21`, `47`, `72`, `299`)
    - `host`: Hostname string (e.g., `pop-nyc`, `pop-fra`, `edge-03`, `edge-02`)
    - `status`: Pill/Badge with HTTP status codes in red text/light red background (e.g., `504`, `502`, `403`, `500`, `429`)

---

## 4. UI/UX & Styling Guidelines
- **Color Palette**: Clean, crisp light mode (Slate/Gray border lines, white background `#FFFFFF`, soft gray panels `#F8FAFC`). Red error badges (`#EF4444` background / text), blue interactive accents (`#3B82F6` or `#2563EB`), green run button (`#10B981`).
- **Typography**: Monospace font for timestamps, log messages, query inputs, and status codes. Clean sans-serif font (Inter / Roboto) for UI navigation and field labels.
- **Interactivity**:
  - Table rows should highlight on hover.
  - Histogram bars should show tooltips on hover.
  - Sidebar fields should highlight with `+` / `-` filter buttons when hovered.
