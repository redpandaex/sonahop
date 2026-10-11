# SonaHop UI

Shared Windows/Android interface powered by Tauri 2 and React.

Run commands from the repository root:

- `pnpm dev`: launch the Windows app in development mode.
- `pnpm web:dev`: browser layout preview at http://localhost:1420.
- `pnpm desktop:build`: build the Windows executable.
- `pnpm android:build`: build the ARM64 debug APK.
- `pnpm test` and `pnpm typecheck`: verify preferences and TypeScript.

This stage enumerates real Windows output devices and stores local preferences.
Audio connections, playback and microphone capture are not enabled yet.