# SonaHop Android permission and audio adapter

The Tauri plugin provides Android LAN permission checks and native foreground audio services.
It supports local microphone capture, one-session MediaProjection playback capture, and audio playback through the JNI PCM bridge.
Microphone and playback-capture consent remain explicit; notification stop cancels audio and pending requests.

The shared Rust link core owns discovery and network sockets. The plugin does not install drivers or change OS defaults.
See the [project README](../../README.md) for supported flows and build instructions.
