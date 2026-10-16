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
- Windows may share system audio and receive one selected Android microphone concurrently. An Android client may listen and upload its microphone concurrently; a second tablet may also listen.
- Initial microphone support uses an existing user-installed Windows virtual audio path.
- USB is available for deployment/diagnostics; dedicated USB audio is a later scope decision.

## Current stage
- Follow docs/superpowers/plans/2026-10-13-tag-release.md for the release-workflow stage; the completed optional-device/duplex stage is recorded in docs/superpowers/plans/2026-10-13-optional-devices-duplex.md; the role architecture remains in docs/superpowers/specs/2026-10-12-multi-role-audio-design.md. Earlier UI, LAN, exclusive/independent audio and diagnostics documents record historical stages.
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
- Android hosting, phone-to-phone audio and Windows player are this stage's authorized scope. Role and platform are independent; role switching stops old audio/connections. Servers support two players. A Windows host can receive one remote microphone while sharing; an Android client may concurrently listen and upload one microphone.
- Android playback capture must not request audio focus or record the hardware microphone. Obtain fresh consent for every start, run the mediaProjection foreground service first, and release when revoked/stopped. JNI callbacks must match their session and mono/stereo packet size.
- Windows microphone capture requires an available recording endpoint; no device means a disabled input choice and a clear explanation. Do not create virtual devices or install drivers.

- The owner authorized automatic local-network discovery on app startup and after role changes, removal of normal network toggles, shared-device IPv4 display, Android single-app audio selection and shared/listen/settings bottom navigation. Pairing and audio still require user action. TUN is not a confirmed cause; do not alter system VPN, routes or firewall. Microphone squeal remains deferred.

- Latest UI feedback supersedes the earlier Android app-selector design: remove custom capture-scope/app lists, UID filters, package-query command and preferences entirely, using Android's normal consent and app-selection flow. Do not claim that system screen selection guarantees additional audio UID isolation. Keep all local addresses after the owner withdrew WLAN-only filtering. Simplify visible explanations, improve dark contrast, and keep desktop scrolling inside the main content area. The owner confirmed Windows/Android discovery works in 0.4.0; preserve that network implementation.

- Latest clarification supersedes the all-address presentation rule on Windows only: display Wi-Fi interface addresses identified by IEEE80211 type; Android keeps all previous eligible addresses. Discovery and TCP continue to use all eligible interfaces. Display/copy/manual entry are IP-only with a fixed internal port; migrate legacy saved/pasted endpoints. Text fields use one subtle focus treatment, preserving keyboard and error visibility. AudioRelay microphone docs inform the existing phone-host/Windows-client/virtual-cable workflow only; no new driver, automatic virtual-device selection or OS default change is authorized.

- Latest user cancelled own kernel-driver work and authorized VB-CABLE integration plus finishing the existing duplex stage. Reuse WASAPI, recommend detected VB-CABLE/Voicemeeter endpoint pairs, allow explicit advanced routing, and provide scoped official links. Driver installation remains user-operated and independent of the app installer. No AudioRelay proprietary interface dependency. Package current exe/APK/NSIS, review, commit and push the existing branch; user performs runtime tests.

- Latest owner confirmed release only on semantic-version tag pushes whose commit belongs to main history, with fixed Android release signing. Owner explicitly defers key generation and GitHub Secrets setup; do not create/upload keys, merge main, create tags or execute remote publication in this turn. Add fail-fast required Secrets, validate/type-check/compile release tooling, review, commit and push the existing branch. Keep runtime/device tests owner-run.

- Release dependency restoration must use the real pinned pnpm CLI, with no pm_on_fail=ignore wrapper override. Regenerate packageManagerDependencies through pnpm, preserve frozen-lockfile in CI, and validate a clean dependency restore with an unchanged lockfile hash before another release attempt.

- Latest owner explicitly authorized finishing this CI release: from 0.5.2 onward, fix remaining CI-only issues, increment patch versions, commit/push main and create/push new release tags until publication succeeds. Preserve existing tags and fixed signing Secrets. This supersedes earlier per-turn deferrals of version/tag/publication actions; runtime/device tests remain owner-run.
