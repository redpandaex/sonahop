# SonaHop LAN permission adapter

Tauri plugin created with the official CLI. On Android 17 / API 37 and later,
ensure_permission requests ACCESS_LOCAL_NETWORK only after a user starts discovery
or connection. On older Android it returns the device name without requesting an
irrelevant permission. This adapter does not start sockets, record audio or change OS settings.
