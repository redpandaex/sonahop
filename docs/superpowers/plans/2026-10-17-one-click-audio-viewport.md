# One-click audio and viewport implementation plan

> For agentic workers: use superpowers:subagent-driven-development for independent protocol/viewport tasks; coordinator implements UI lifecycle integration. User-run physical/runtime tests override automatic test execution.

**Goal:** Remove PIN and connection approval; make sharing ready by default and connect-to-listen one action; fix desktop double-scroll and Android status-bar overlap; show semantic connection/audio colors.
**Architecture:** Protocol v3 uses fresh anonymous X25519 + existing transcript-bound HKDF/AES-GCM. The frontend coordinates real connection completion and explicit source permissions. One bounded viewport owns each platform's content scroll; native Android parent owns system-bar/cutout insets.
**Spec:** User's accepted in-chat flow on 2026-10-17, including correction that hosts default-share and players reuse connect/disconnect.

## Constraints
- No fixed/hidden PIN, plaintext fallback, manual peer approval or automatic microphone restoration.
- Windows system sound can prepare automatically in host mode; Android system capture still requires per-session system consent. Selecting microphone is explicit, following AudioRelay's microphone-source -> receiver microphone-output flow.
- Maximum two listeners, encrypted frames, rate/handshake limits and explicit microphone grant/virtual-route selection remain.
- A player click records one listen intent, prepares playback from that action, waits for the real peer, starts once; failure/cancel/stop/disconnect/role switch clear intent.
- Disconnect stops received audio and microphone lanes; ordinary listening has no separate start step.
- One main scroll region; source/virtual-device selectors may have bounded popup scrolling. Mobile header and bottom tabs stay outside content scrolling.
- Gray idle, blue available/discovering, amber connecting/waiting, green active/connected, red errors; use state fields, not Chinese-label parsing.
- No automated/runtime/browser/device/audio tests or driver/default-device changes. CLI dependency management, static types and necessary compiler checks only.
- New product scope: no automatic version bump, commit, push, tag or release without a separate concrete delivery instruction.

## Tasks
- [x] 1. Protocol: replace SPAKE2 with fresh X25519, bump wire version, remove PIN/request/answer state and Tauri APIs, autoaccept bounded connections after encrypted confirmation. Files: crates/sonahop-link, apps/sonahop-ui/src-tauri/src/link_commands.rs and lib.rs; dependency changes through Cargo CLI.
- [x] 2. Viewport: root locked height/overflow, desktop sole desktop-main scroller, mobile header/main/tabs three-row layout, native systemBars|displayCutout padding and zero only handled insets. Files: App.css and Android MainActivity.kt (index/App only if essential).
- [x] 3. UI/lifecycle: remove PIN and request UI/types, connect/disconnect controls, automatic host system-source preparation and client listen intent, explicit microphone-source start, semantic badge/footer colors. Files: use-link/use-workbench/use-audio, bridges, controls, audio-controls, lan-connection, screens.
- [x] 4. Integrate, static/compile checks, scoped task reviews and final fresh review; update docs and give concise user acceptance steps.

## Review focus
- Concurrent third client rejected atomically; failure frees reservations; old v2 clients fail clearly without downgrade.
- Navigation/cancel/disconnect and stale polls must not start a superseded audio session; permission rejection must not trigger a retry loop.
- No host microphone capture merely from opening the app or reconnecting; selecting microphone is the explicit action.
- Portrait/landscape, cutout, status/navigation bars and keyboard do not double-apply insets; ordinary pages have one scroll region.
- State labels/colors agree through connecting, waiting source, active lanes, errors and stopped sessions.
- Pausing/resuming or changing the host source preserves listening subscriptions and each player's selected output. Reuse the existing receiving worker/sink while paused (silence without a packet timeout), reset only the decoder for the authorized new stream, and reject queued old-stream PCM. This keeps Android's ongoing foreground playback session through background/lock-screen recovery; full stop/notification stop still clears it. A completed/cleanly closed connection attempt cannot leave a pending frontend intent.

## User acceptance (pending)
1. Update both platforms together; connect without PIN and hear the default shared source; disconnect stops playback. Pause/resume and change the host source while connected (including a pause longer than 12 seconds with Android backgrounded/locked); audio resumes through the same selected output. Rapidly disconnect during connection; the player returns to discovery.
2. Phone share app source obtains normal Android consent; microphone source is explicit and feeds the configured computer microphone endpoint.
3. Expand Windows microphone details: one scrollbar; phone scroll: header/tabs stay below/above system bars.
4. Observe gray/blue/amber/green/red for actual corresponding states in light/dark modes.

## Verification and review record
- Protocol dependency/API changes and viewport/native changes each received a separate static review; no Critical/Important issues remained in those scopes.
- Final whole-change review found two Important state-lifecycle issues; both were corrected. A directly associated Android background-resume issue was also corrected by reusing the existing receiving sink rather than restarting Android playback. Final scoped recheck reported no remaining Critical/Important issue in those fixes.
- TypeScript tsc --noEmit, cargo check --package sonahop-link --tests --locked (compile test sources only), cargo check --package sonahop-ui --locked, cargo fmt --all -- --check, Android :app:compileArm64DebugKotlin with Rust packaging excluded, and git diff --check succeeded.
- Targeted frontend Biome check succeeded with no errors; one existing void-union warning and three existing template-string infos remain. No zero-warning claim is made.
- No automated tests, applications, browser/device interactions, real audio or release packaging ran. Physical acceptance above remains pending; product versions/tags remain unchanged and this stage is uncommitted/unpublished.

## Local delivery authorized and completed 2026-10-18
- The owner explicitly requested builds after the source-check stage. Generated Windows release EXE, NSIS installer and an ARM64 Release APK in artifacts/one-click-audio.
- Used real pinned pnpm 11.25.0; both frontend production builds passed. Windows Rust release completed in 30.97s and Android Rust release in 29.27s. Windows symbol-link restrictions triggered the existing same-build-library Gradle fallback; APK packaging passed in 46s.
- APK signing uses the user's existing ignored PKCS12 key. v2/v3 signatures verify and certificate SHA256 matches the previous v0.5.3 Release. AAPT confirms package io.github.redpandaex.sonahop, version 0.5.3/versionCode 5003, ARM64, and non-debuggable release. Windows copied binary/installer hashes match their build outputs; all three delivered asset hashes match build-info.json and SHA256SUMS.txt.
- Source base is 85a2100 plus the uncommitted one-click changes. Local preview retains app version 0.5.3 and wire protocol v3; both devices must use the local update. No tag/commit/push/publication, test run, app launch or device/audio validation occurred.
- Deliverable paths: artifacts/one-click-audio/sonahop-windows.exe; sonahop-windows-setup.exe; sonahop-android-arm64-release.apk. Build metadata, signature verification record, checksums and logs are alongside them. Physical acceptance remains pending.

## Acceptance follow-up and local delivery 2026-10-18
- Owner reported checks1/3/6/7/8/9 OK; check4 function OK but manual recording-end label touched the expansion button. Check2 showed normal PC disconnect/cancel sometimes described as remote closure. Check5 was clarified by screenshot and follow-up: PC local playback/headphones, not virtual microphone; Type-C headphone playback is normal and piercing noise essentially disappears. No audio algorithm, capture preset or virtual route change was justified/applied.
- Implemented typed Local/Remote/Failed end reasons, intent before audio teardown, idempotent stop, neutral EOF and original transport-error diagnostics. Normal endings stop audio without creating an error. Stop blocks late accepts/connect/publication and allows all existing controls, including pending handshakes, up to2s to reach Bye before generation shutdown. Manual recording selection now has a spaced subsection and separator; the label rule matches DeviceChoice's wrapper.
- Targeted final review found one Important pending-handshake grace omission; coordinator confirmed and fixed the wait condition from connected-only to all controls. Reviewer verified the final source and marked that finding resolved; no other Critical/Important findings remained in that scope. Both platforms were rebuilt after this final change, superseding the intermediate build.
- TypeScript/Biome, core/app Rust checks, rustfmt and diff checks passed. Final Windows release/NSIS, Android ARM64 release packaging and existing-key v2/v3 signing succeeded. Delivery is artifacts/acceptance-polish with version0.5.3, wirev3, unchanged signing identity. Metadata/checksums record the final asset hashes. No tests, app/device actions, commits, pushes, tags or publication. Updated disconnect/cancel and layout behavior await user regression.

## Saved-device module removal 2026-10-18
- Owner explicitly chose to remove the saved shared-device module and not remember the last connection address. Removed SavedServer and details, computerName/computerAddress preference fields, restore shortcut and dedicated styles. Both platforms retain automatic discovery and a transient manual IP input; it starts blank and clears after connected. Old bookmark data is ignored by the preference reader and removed during the existing preference save, while other audio/theme choices remain.
- Updated existing preference test fixtures for current audio fields; did not add or run automated tests. Source reference scan found zero saved-module/name/address preference references. Targeted Biome and TypeScript passed (one existing template-string info); both frontend production builds and native Windows/Android release packaging passed. Existing fixed Android key v2/v3 verification passed. No device/app tests or publication/version changes.
- Latest local artifacts are in artifacts/ui-simplified, retaining app0.5.3/wire3; metadata and SHA256SUMS.txt record hashes. User acceptance: saved section absent, manual entry/discovery still work, successful connection/reopen does not restore a remote address. Existing unrelated .VSCodeCounter files were left untouched.

## Final review, cleanup and integration
- Owner accepted the delivered UI and authorized final review plus commit/push on main, then asked to prune unnecessary files. Fresh final review reported no Critical/Important blocker after inspecting the complete current implementation and cleanup; only whitespace cleanup remained and was formatted.
- Removed nine generated editor statistics reports, eighteen unused iOS PNG icon variants, the stale duplicate UI README, two unused legacy UI exports and twenty-two exclusive CSS rules. Retained configured Windows and Android assets. Consolidated project guidance, corrected the native adapter README and replaced two new let-chains with equivalent nested if for the core's declared Rust minimum.
- Fresh checks after cleanup: frontend TypeScript/production build, cargo check --workspace --all-targets --locked --offline (compile test sources only), cargo fmt --all -- --check and git diff --check passed. Targeted Biome had zero errors with the same pre-existing void-union warning/three template infos. No automated/runtime/device checks were run by the agent.
- Remote main was fetched and matched local base85a2100 before integration. Stage only reviewed source/documentation/configuration and intended deletions; signing material, dependencies, build outputs and generated reports stay ignored. Integration is an ordinary main push; no version bump, tag or Release is requested.
