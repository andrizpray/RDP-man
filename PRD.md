# RDP Man — Product Requirements Document

> Cross-platform RDP server monitor & multi-session client for IT admins.

**Version:** 1.0 Final  
**Date:** 2026-07-13  
**Author:** Andriz  
**Status:** Ready for implementation

---

## 1. Problem

IT admin managing 1-20 RDP servers needs one app to monitor status, connect to any server instantly, and run multiple sessions simultaneously. Today: open RDP client one-by-one, no dashboard, no history.

## 2. Solution

**RDP Man** — desktop app with live server dashboard, embedded multi-session RDP client (tabs + tiled view), and local connection history.

## 3. Target User

| Attribute | Value |
|-----------|-------|
| Role | IT Admin / Sysadmin |
| Context | Server farm monitoring (1-20 RDP servers) |
| Platforms | Windows 10+, Ubuntu 20.04+, macOS 12+ |
| Users | Single user, no auth |

## 4. Tech Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Framework | **Tauri 2.0** | Cross-platform, ~10MB binary, Rust backend |
| Language | **Rust** | Safety + performance for concurrent RDP sessions |
| Frontend | **React + TypeScript** | Dashboard, tabs, tiled layout |
| RDP Engine | **FreeRDP** (C library via FFI) | Battle-tested, full feature set |
| Database | **SQLite** (rusqlite) | Embedded, zero-config, local-only |
| State | **Zustand** | Minimal, no boilerplate |

### FreeRDP Integration

- Each RDP session = 1 `freerdp*` instance in its own Rust thread
- Framebuffer blitted to `<canvas>` via Tauri IPC (binary channel)
- Input events (keyboard/mouse) forwarded back to FreeRDP
- Built as native library (`libfreerdp-client`), linked via `cc` crate or `bindgen`

## 5. Features

### 5.1 Connection Manager
- CRUD for RDP connection profiles
- Fields: `display_name`, `hostname`, `port` (default 3389), `username`, `password`
- Password stored in **plain text** (SQLite, no encryption)
- Flat list — no grouping, no tags

### 5.2 Server Dashboard
- Grid view of all registered servers
- Status badge per server: 🟢 online / 🔴 offline / ⚪ unknown
- Health check: TCP probe to `hostname:port`, interval configurable (default 30s)
- Click → open RDP session in new tab

### 5.3 Multi-Session RDP Viewer
- **Tab mode**: browser-like tabs, one session per tab (default view)
- **Tile mode**: split screen, max **10 sessions** visible simultaneously (2x5, 5x2, 3x4, etc. — user adjustable)
- Tab bar shows: server name + status indicator + close button
- Drag to reorder tabs
- Open/close sessions without app restart

### 5.4 RDP Feature Set (all enabled)

| Feature | Status |
|---------|--------|
| Screen rendering (GFX/H.264) | ✅ |
| Keyboard & mouse input | ✅ |
| Clipboard sharing (bidirectional) | ✅ |
| Local drive redirection | ✅ |
| Audio redirection (remote → local) | ✅ |
| Printer redirection | ✅ |
| Multi-monitor (host side) | ✅ |
| NLA (Network Level Authentication) | ✅ |

### 5.5 Auto-Reconnect
- On disconnect: auto-retry with **exponential backoff** (1s → 2s → 4s → 8s → max 30s)
- Max retries: 10, then show "reconnect failed" with manual retry button
- Visual indicator on tab when reconnecting

### 5.6 Session History / Log
- Auto-log every connection attempt:
  ```
  id | connection_id | hostname | username | connected_at | disconnected_at | duration_sec | status
  ```
- Status: `connected`, `disconnected`, `error`, `reconnecting`
- View in-app: table with sort by date, filter by server
- **No export.** Local SQLite only.
- Retention: keep all (no auto-delete, disk usage negligible)

## 6. Data Model

```sql
CREATE TABLE connections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    display_name TEXT NOT NULL,
    hostname     TEXT NOT NULL,
    port         INTEGER NOT NULL DEFAULT 3389,
    username     TEXT NOT NULL,
    password     TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE sessions_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id   INTEGER NOT NULL REFERENCES connections(id),
    hostname        TEXT NOT NULL,
    username        TEXT NOT NULL,
    connected_at    TEXT NOT NULL,
    disconnected_at TEXT,
    duration_sec    INTEGER,
    status          TEXT NOT NULL CHECK(status IN ('connected','disconnected','error','reconnecting'))
);
```

## 7. UI Wireframes (Text)

### Dashboard (default view)
```
┌──────────────────────────────────────────────────────────┐
│  RDP Man                                    [+] Add      │
├──────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐    │
│  │ WEB-01  │  │ DB-01   │  │ APP-01  │  │ APP-02  │    │
│  │ 🟢      │  │ 🟢      │  │ 🔴      │  │ 🟢      │    │
│  │ 10.0.1.1│  │ 10.0.1.2│  │ 10.0.1.3│  │ 10.0.1.4│    │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘    │
│                                                          │
│  ┌─────────┐  ┌─────────┐                               │
│  │ CACHE-01│  │ LB-01   │     ...                       │
│  │ ⚪      │  │ 🟢      │                               │
│  └─────────┘  └─────────┘                               │
├──────────────────────────────────────────────────────────┤
│  History                                    [View Logs]  │
└──────────────────────────────────────────────────────────┘
```

### Multi-Session (tile mode)
```
┌──────────────────────────────────────────────────────────┐
│  [WEB-01 ×] [DB-01 ×] [APP-01 ×] [+] Tile|Tab          │
├──────────────────────────────┬───────────────────────────┤
│                              │                           │
│      RDP Session: WEB-01     │   RDP Session: DB-01      │
│      (live framebuffer)      │   (live framebuffer)      │
│                              │                           │
├──────────────────────────────┼───────────────────────────┤
│                              │                           │
│      RDP Session: APP-01     │   (empty tile)            │
│      (reconnecting 3/10...)  │                           │
│                              │                           │
└──────────────────────────────┴───────────────────────────┘
```

## 8. Non-Functional

| Requirement | Target |
|-------------|--------|
| App size | < 15MB installer |
| Memory (idle, 1 session) | < 300MB |
| Memory (10 sessions) | < 1.5GB |
| Startup time | < 3s |
| RDP frame rate | ≥ 24fps per session (GFX mode) |
| Reconnect latency | 1s initial, 30s max backoff |

## 9. Architecture

```
┌─────────────────────────────────────────────┐
│              Tauri WebView (React)           │
│  ┌─────────┐  ┌──────────┐  ┌───────────┐  │
│  │Dashboard │  │ Session  │  │  History   │  │
│  │  View    │  │ Viewer   │  │   View     │  │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  │
│       │              │              │         │
│       └──────┬───────┴──────┬───────┘         │
│              │ Tauri IPC    │                  │
├──────────────┴──────────────┴─────────────────┤
│              Rust Backend (Tauri)              │
│  ┌──────────┐ ┌───────────┐ ┌─────────────┐  │
│  │Connection│ │  Session  │ │   Logger    │  │
│  │ Manager  │ │  Manager  │ │  (SQLite)   │  │
│  └────┬─────┘ └─────┬─────┘ └─────────────┘  │
│       │              │                         │
│       │    ┌─────────┴──────────┐              │
│       │    │   FreeRDP FFI      │              │
│       │    │ ┌────┐┌────┐┌────┐ │              │
│       │    │ │S1  ││S2  ││S3  │ │              │
│       │    │ │thrd││thrd││thrd│ │  (up to 10)  │
│       │    │ └────┘└────┘└────┘ │              │
│       │    └────────────────────┘              │
│       │                                        │
│  ┌────┴─────────────┐                         │
│  │   Health Checker  │ (TCP probe, async)      │
│  └──────────────────┘                         │
└───────────────────────────────────────────────┘
```

## 10. Out of Scope (v1)

- Encryption / credential vault
- Multi-user / auth / RBAC
- Export / import (config or logs)
- Server grouping / tags
- Alerting (email, webhook, push)
- SSH / VNC support
- Gateway / broker / HA
- Drag-and-drop file transfer
- Session recording / playback

## 11. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| FreeRDP FFI build complexity | High — blocks everything | Use `cc` crate to compile FreeRDP from source, CI builds for 3 platforms |
| Canvas blit perf at 10 sessions | Medium — may lag | Dirty-rect rendering (only blit changed regions), throttle to 30fps |
| macOS FreeRDP support | Medium — macOS less tested upstream | Test early, fallback to `xfreerdp` subprocess if FFI fails |
| Clipboard/drive integration via FFI | Medium — complex channel protocol | Implement basic first (clipboard), add drive/audio iteratively |

---

*End of PRD. Ready for implementation.*
