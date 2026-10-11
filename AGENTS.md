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
- Follow docs/superpowers/specs/2026-10-10-desktop-android-ui-design.md and the matching stage plan.
- Deliver native Windows and Android UI previews before network audio. Windows device enumeration is real; browser examples must stay explicitly labeled.
- Save preferences locally. Do not claim connection, playback, microphone permission, remote gain or independent audio has been implemented.
- Windows offers light/dark/system appearance; Android follows its system theme.
- The owner reports Windows and Y700 both launch successfully. Discovery, connection and audio are not implemented yet; no further GUI startup verification is needed now.
- Keep PC-mute-independent receiving volume as the next audio stage acceptance requirement.
- Local WAV capture remains an explicit diagnostic operation. Ask before recording real system audio; synthetic tests and output-device enumeration are allowed.
## Frontend tooling
- Use Biome for formatting/lint and Tailwind CSS 4 with the Vite plugin for styles. shadcn/ui Radix components live in apps/sonahop-ui/src/components/ui.
- Use the official shadcn CLI to add components; keep the current data-theme light/dark/system behavior and native palette.
- The owner runs tests and interactive validation personally. After a reviewable change, provide concise verification steps and wait for feedback; do not launch test/build/device verification automatically.
