# SonaHop project guidance

## Working agreements
- Communicate in Chinese and use Superpowers as the active development workflow.
- Use only lightweight Superpowers stage documents for planning; do not introduce a second specification workflow.
- Prefer official CLI commands to scaffold projects and manage dependencies: cargo new/add/remove and pnpm create/add/remove.
- Manually edit only configuration the CLI does not cover and application implementation.
- When the owner explicitly requests a plan and execution together, create a reviewable plan and execute the authorized scope without repeated process approval. Ask about material scope/acceptance uncertainty or sensitive real-device actions.
- Use small stage plans and implement the authorized scope directly. The owner currently prefers to run tests and interactive verification personally: do not run automated tests, browser/device tests or long verification builds unless explicitly authorized. When validation is needed, stop and provide concise commands and acceptance steps.
- Keep actual build/test evidence separate from unverified physical-device claims.
- Do not copy or use SoundPush as an architecture or implementation reference.

## Architecture
- Shared Rust core is independent of OS APIs and Tauri.
- Platform backends depend on core interfaces; application entry points compose them.
- Add real platform implementations progressively rather than empty claimed support.
- Tauri remains the planned desktop/mobile UI; native Android services own background audio.
- Fixed codec settings, bounded queues and diagnostics precede adaptive optimization.

## Confirmed product milestone
- Windows 11 host; Xiaomi 17 Pro Max and Lenovo Y700 generation 4, both Android 16.
- Listen mode supports two Android listeners.
- Windows may share system audio and receive one selected Android microphone concurrently. Each Android still runs only one audio direction; a second tablet can listen while a phone provides the microphone.
- Initial microphone support uses an existing user-installed Windows virtual audio path.
- USB is available for deployment/diagnostics; dedicated USB audio is a later scope decision.

## Current stage
- Follow docs/superpowers/plans/2026-10-12-discovery-ui.md for the current discovery/UI stage; the role architecture remains in docs/superpowers/specs/2026-10-12-multi-role-audio-design.md. Earlier UI, LAN, exclusive/independent audio and diagnostics documents record historical stages.
- The owner authorized completing the confirmed audio MVP overnight and will run tests in the morning.
- Current source includes authenticated LAN + audio. Final Windows release, Android ARM64 debug and frontend production builds succeeded; APK signature/permissions/JNI exports were checked. No automated tests, app execution or physical-device/audio verification ran. Do not equate compilation with working device proof.
- Old artifacts/ui-preview binaries do not include the new LAN/audio source. Use scripts/build-audio-mvp.ps1 when the owner runs packaging; do not silently substitute old binaries.
- Windows and Android may each host or play. Server source is explicitly system playback or a local microphone. Windows process loopback remains the system source; Android uses one-time MediaProjection playback capture. Player output is local and never changes OS defaults. The WAV CLI remains diagnostic.
- No automatic microphone restart. Connection, approval, user audio start, hardware initialization, error and stop states must be real.
- Virtual audio render sink selection is explicit and uses user-installed software; never auto-select a physical speaker.
- The latest overnight request authorizes final exe/APK packaging so the owner can test tomorrow. Compile/package and repair compilation errors; do not run automated tests, launch apps, or interact with audio/devices. Earlier user-run-only build guidance applies to intermediate verification, not this final delivery.
## Frontend tooling
- Use Biome for formatting/lint and Tailwind CSS 4 with the Vite plugin for styles. shadcn/ui Radix components live in apps/sonahop-ui/src/components/ui.
- Use the official shadcn CLI to add components; keep the current data-theme light/dark/system behavior and native palette.
- The owner runs tests and interactive validation personally. Final packaging is authorized by the latest overnight request; do not launch tests, apps or device/audio verification automatically. Provide concise manual acceptance steps with the built artifacts.

## Current accepted changes
- The owner asked to implement volume-control removal, default-off Windows autostart and independent Windows listen/microphone lanes, then they will test.
- The owner reopened disconnection work with send-blocked/heartbeat-timeout screenshots. Follow docs/superpowers/plans/2026-10-12-link-diagnostics.md; implement bounded logs and decouple connection reads from writes. Device verification remains owner-run.
- Microphone monitoring/mixing is explicitly deferred: keep the phone microphone and Windows system-audio stream independent.
- Native Windows virtual playback/capture endpoints remain a future driver-backed milestone. Do not install drivers or change OS defaults; record the default-off, capability-gated proposal.
- Receiving playback uses unit gain and the Android media-volume keys; microphone gain is retained.
- Autostart registration happens only after a user toggle, never automatically during app startup or agent verification.
- Android hosting, phone-to-phone audio and Windows player are this stage's authorized scope. Role and platform are independent; role switching stops old audio/connections. Servers support two players. Each player uses one audio direction at a time; Windows host can additionally receive one remote microphone.
- Android playback capture must not request audio focus or record the hardware microphone. Obtain fresh consent for every start, run the mediaProjection foreground service first, and release when revoked/stopped. JNI callbacks must match their session and mono/stereo packet size.
- Windows microphone capture requires an available recording endpoint; no device means a disabled input choice and a clear explanation. Do not create virtual devices or install drivers.

- The owner authorized automatic local-network discovery on app startup and after role changes, removal of normal network toggles, shared-device IPv4 display, Android single-app audio selection and shared/listen/settings bottom navigation. Pairing and audio still require user action. TUN is not a confirmed cause; do not alter system VPN, routes or firewall. Microphone squeal remains deferred.
