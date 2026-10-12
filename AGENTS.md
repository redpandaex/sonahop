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
- Microphone mode accepts one selected Android and pauses all listening.
- Initial microphone support uses an existing user-installed Windows virtual audio path.
- USB is available for deployment/diagnostics; dedicated USB audio is a later scope decision.

## Current stage
- Follow docs/superpowers/specs/2026-10-12-audio-mvp.md and the matching plan. Earlier UI and protocol-v1 LAN documents are historical stages.
- The owner authorized completing the confirmed audio MVP overnight and will run tests in the morning.
- Current source includes authenticated LAN + audio. Final Windows release, Android ARM64 debug and frontend production builds succeeded; APK signature/permissions/JNI exports were checked. No automated tests, app execution or physical-device/audio verification ran. Do not equate compilation with working device proof.
- Old artifacts/ui-preview binaries do not include the new LAN/audio source. Use scripts/build-audio-mvp.ps1 when the owner runs packaging; do not silently substitute old binaries.
- Windows process loopback is the streaming source; the existing endpoint WAV CLI remains diagnostic. Android alone supplies microphone input; do not change system volume/output route, install drivers or modify firewall.
- No automatic microphone restart. Connection, approval, user audio start, hardware initialization, error and stop states must be real.
- Virtual audio render sink selection is explicit and uses user-installed software; never auto-select a physical speaker.
- The latest overnight request authorizes final exe/APK packaging so the owner can test tomorrow. Compile/package and repair compilation errors; do not run automated tests, launch apps, or interact with audio/devices. Earlier user-run-only build guidance applies to intermediate verification, not this final delivery.
## Frontend tooling
- Use Biome for formatting/lint and Tailwind CSS 4 with the Vite plugin for styles. shadcn/ui Radix components live in apps/sonahop-ui/src/components/ui.
- Use the official shadcn CLI to add components; keep the current data-theme light/dark/system behavior and native palette.
- The owner runs tests and interactive validation personally. Final packaging is authorized by the latest overnight request; do not launch tests, apps or device/audio verification automatically. Provide concise manual acceptance steps with the built artifacts.
