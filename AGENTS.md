# SonaHop project guidance

## Working agreements
- Communicate in Chinese and use Superpowers as the active development workflow.
- Existing OpenSpec files are retained as reference for agreed requirements. Do not maintain a second parallel workflow or invoke OpenSpec unless the owner explicitly asks.
- Prefer official CLI commands to scaffold projects and manage dependencies: cargo new/add/remove and pnpm create/add/remove.
- Manually edit only configuration the CLI does not cover and application implementation.
- When the owner explicitly requests a plan and execution together, create a reviewable plan and execute the authorized scope without repeated process approval. Ask about material scope/acceptance uncertainty or sensitive real-device actions.
- Use small stage plans. Implement within this session; meaningful feature tests precede implementation, and generated templates/configuration are verified by appropriate build commands.
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
- The owner chose APK delivery only this round; physical Y700 installation/testing is deferred.
- Keep PC-mute-independent receiving volume as the next audio stage acceptance requirement.
- Local WAV capture remains an explicit diagnostic operation. Ask before recording real system audio; synthetic tests and output-device enumeration are allowed.