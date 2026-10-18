# SonaHop project guidance

## Working agreements
- Communicate in Chinese. Use lightweight Superpowers stage documents; do not introduce a second specification workflow.
- Prefer official Cargo/Tauri/pnpm/shadcn CLI operations for scaffolding and dependencies. Edit implementation and unsupported configuration normally.
- Execute already-authorized scope without repeated process approval; clarify material device/acceptance uncertainty.
- The owner runs automated tests, browser/app/device/audio checks personally. Static type/compiler checks and explicitly requested packaging are allowed; never equate compilation with physical acceptance.
- Do not use SoundPush as an architecture or implementation reference.

## Architecture and product scope
- Shared Rust core is independent of OS APIs and Tauri. Platform adapters implement real audio; Tauri composes UI and services.
- Windows and Android may each share or listen. Confirmed devices: Windows 11, Xiaomi 17 Pro Max and Lenovo Y700 generation 4 on Android 16.
- One host supports at most two listeners. System audio and one independently approved remote microphone may run concurrently; an Android listener may also upload its microphone.
- Native Android foreground services own background audio. Internal playback capture requires fresh MediaProjection consent each start, excludes this app, and must not record the hardware microphone or request audio focus.
- Microphone capture is explicit. Never restore recording after reopening, reconnecting, permission revocation or notification stop.
- Windows sharing uses process loopback. Receiving uses unit gain/system device volume; microphone gain is separate.
- Microphone monitoring/mixing and feedback suppression remain deferred. The owner confirmed phone-mic playback through PC Type-C headphones is normal; do not misclassify that scenario as virtual-device failure.
- Use existing user-installed virtual endpoints with explicit playback/recording pairing. Never install/bundle drivers, select physical speakers as virtual microphones, change OS defaults, VPN/routes/firewall, or depend on AudioRelay proprietary interfaces.
- Windows autostart defaults off and changes only through the user's toggle.
- Windows tray: persistent close-to-tray/exit choice, live status header/icon/tooltip, and native actions while hidden. Host Start shares system audio; Stop ends capture, all peers and discovery. Client Connect opens device selection; Disconnect ends audio/connection and resumes discovery. Generic tray share never starts a microphone. Serialize operations, reject superseded UI commands, and update native menus only outside OperationGate. Missing tray/failed hide falls back to normal exit.

## Current interaction and lifecycle
- Wire protocol v3 uses fresh anonymous X25519, transcript-bound HKDF and directional AES-GCM. No PIN or connection approval; encryption does not authenticate device identity.
- Preserve handshake/rate/frame bounds, atomic two-peer reservations, counter/replay protection and explicit remote microphone approval.
- Discovery starts automatically with the app/role. Explicit tray StopSharing stays stopped until a user starts sharing again. Player connect waits for a real peer before one-shot listen; disconnect stops its lanes.
- Hosts prepare system/app audio by default; Android consent remains mandatory. A saved microphone choice does not start capture automatically.
- Host pause/source changes retain listener subscriptions and the receiver's existing sink. Reset only the decoder for the authorized new stream; discard old stream packets/PCM. Full/notification stop is terminal.
- Record Local/Remote/Failed stop reasons before audio teardown. Allow all existing controls, including pending handshakes, at most two seconds for Bye before generation shutdown.
- Keep lifecycle/epoch guards against late starts, failed/cancelled intents and retry loops. An ordinary listener leaving does not stop the host's chosen source session.
- No saved shared-device/address-book module or last-address memory. Manual IP is transient, initially blank and cleared after connection; old bookmark preference fields are dropped while audio/theme preferences remain.
- Display/copy/manual entry use IP only with fixed internal ports. Accept pasted legacy IP:port syntax. Windows displays IEEE80211 Wi-Fi addresses; Android retains eligible addresses. Display filtering must not restrict discovery/socket interfaces.
- One bounded main content scroller per platform. Android native parent owns system-bar/cutout padding and clears only handled insets; preserve IME dispatch.
- Status colors follow actual state: gray idle, blue discovering, amber connecting/waiting, green active, red error.

## Tooling and delivery
- Frontend uses Biome, Tailwind CSS 4 and existing shadcn/Radix components. Run UI Biome from apps/sonahop-ui to avoid nested root configuration errors.
- Use the real pinned pnpm CLI (11.25.0), not a wrapper setting pm_on_fail=ignore. Preserve frozen-lockfile checks and CLI-generated package-manager lock metadata.
- Use scripts/build-audio-mvp.ps1 for Windows/Android packaging. The symbol-link fallback may use only the library from that same successful Rust build.
- Keep signing material in ignored local paths and pass passwords through environment names; never print, commit or upload secrets. Android release signing uses the existing fixed certificate.
- Releases v0.5.4+ use wire protocol v3, incompatible with published v0.5.3. Both devices must use compatible wire-v3 builds; the tray release does not change the wire protocol.
- Latest stateful Windows tray EXE/NSIS and delivery records are under artifacts/windows-tray-actions; artifacts/windows-tray is the superseded Stop-audio preview. Latest local Android signed packages remain under artifacts/ui-simplified. Current tray interactions await owner validation.
- The owner accepted the local tray preview and explicitly authorized committing/pushing the tray work to main and triggering the next patch Release (v0.5.5). Keep existing tags unchanged and use the configured fixed Android signing certificate.
- Release workflow runs only on version-tag pushes whose commit belongs to main history, with fixed Android signing. The earlier automatic CI-retry authorization ended when v0.5.3 was published.
- Preserve build dependencies, target-platform assets, current user data and signing keys when pruning generated/unused files.

## References
- Current stage and acceptance evidence: docs/superpowers/plans/2026-10-17-one-click-audio-viewport.md.
- User/build instructions: README.md. Release process: docs/releases.md.
- Architecture: docs/superpowers/specs/2026-10-12-multi-role-audio-design.md.
- Virtual routing: docs/superpowers/plans/2026-10-13-optional-devices-duplex.md and its linked design.
- Older stage documents record history; current agreements above supersede earlier PIN/manual-start assumptions.
