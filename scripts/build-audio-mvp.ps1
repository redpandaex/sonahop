[CmdletBinding()]
param([ValidateSet('Windows','Android','Both')][string]$Platform='Both', [string]$OutputDirectory='artifacts/audio-mvp', [switch]$WindowsInstaller, [switch]$AndroidRelease)
$ErrorActionPreference='Stop'
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repoRoot
$artifactDir=[IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
if(-not $artifactDir.StartsWith($repoRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw "Artifact directory must be inside the repository." }
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
$environmentNames=@('Path','CMAKE','OPUS_LIB_DIR','OPUS_STATIC','JAVA_HOME','ANDROID_HOME','NDK_HOME')
$previousEnvironment=@{}
foreach($name in $environmentNames) { $previousEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process') }
function Run-Native([string]$Program,[string[]]$Arguments) {
    $savedAction=$ErrorActionPreference; $nativeExit=0
    try { $ErrorActionPreference="Continue"; & $Program @Arguments; $nativeExit=$LASTEXITCODE }
    finally { $ErrorActionPreference=$savedAction }
    if($nativeExit -ne 0) { throw "$Program failed with exit $nativeExit" }
}
function Prepare-Opus([string]$Target) {
    # Fetch source only. No automated tests or audio I/O are performed by this script.
    Run-Native 'cargo' @('fetch','--locked','--target',$(if($Target -eq 'Android'){'aarch64-linux-android'}else{'x86_64-pc-windows-msvc'}))
    $cargoDirectory=if($env:CARGO_HOME){$env:CARGO_HOME}else{Join-Path $env:USERPROFILE '.cargo'}
    $sourceDirectories=Get-ChildItem -LiteralPath (Join-Path $cargoDirectory 'registry/src') -Directory |
        ForEach-Object { Join-Path $_.FullName 'audiopus_sys-0.2.2/opus' } |
        Where-Object { Test-Path -LiteralPath (Join-Path $_ 'CMakeLists.txt') }
    $opusSource=$sourceDirectories | Select-Object -First 1
    if(-not $opusSource) { throw 'Cannot locate the locked audiopus_sys 0.2.2 source after cargo fetch.' }
    $nativeBuild=Join-Path $repoRoot ("target/native-opus/"+$Target.ToLowerInvariant())
    $nativeInstall=Join-Path $nativeBuild 'install'
    $arguments=@('-S',$opusSource,'-B',$nativeBuild,'-DCMAKE_POLICY_VERSION_MINIMUM=3.5',"-DCMAKE_INSTALL_PREFIX=$nativeInstall",'-DOPUS_BUILD_TESTING=OFF','-DOPUS_BUILD_PROGRAMS=OFF','-DOPUS_BUILD_SHARED_LIBRARY=OFF')
    if($Target -eq 'Android') {
        $toolchain=Join-Path $env:NDK_HOME 'build/cmake/android.toolchain.cmake'
        if(-not(Test-Path -LiteralPath $toolchain)) { throw 'Set NDK_HOME to Android NDK 30.0.16248370.' }
        $ninjaPath=if($env:NINJA){$env:NINJA}else{(Get-Command ninja -ErrorAction SilentlyContinue).Source}
        if(-not $ninjaPath -or -not (Test-Path -LiteralPath $ninjaPath -PathType Leaf)) { throw 'Install Ninja or set NINJA to its executable path.' }
        $arguments+=@('-G','Ninja',"-DCMAKE_MAKE_PROGRAM=$ninjaPath","-DCMAKE_TOOLCHAIN_FILE=$toolchain",'-DANDROID_ABI=arm64-v8a','-DANDROID_PLATFORM=android-24','-DCMAKE_BUILD_TYPE=Release')
    } else { $arguments+=@('-A','x64') }
    Run-Native $env:CMAKE $arguments
    Run-Native $env:CMAKE @('--build',$nativeBuild,'--config','Release','--parallel','2','--target','install')
    $env:OPUS_LIB_DIR=$nativeInstall
    $env:OPUS_STATIC='1'
}
try {
    $cmake=Get-Command cmake -ErrorAction SilentlyContinue
    if($cmake) { $env:CMAKE=$cmake.Source } else {
        $vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
        if(-not(Test-Path -LiteralPath $vswhere)) { throw 'Install CMake and the Visual Studio C++ build tools.' }
        $visualStudio=(& $vswhere -latest -products '*' -property installationPath | Select-Object -First 1)
        $cmakeRoot=Join-Path $visualStudio 'Common7/IDE/CommonExtensions/Microsoft/CMake'
        $env:CMAKE=Join-Path $cmakeRoot 'CMake/bin/cmake.exe'
        if(-not(Test-Path -LiteralPath $env:CMAKE)) { throw 'The Visual Studio CMake component is missing.' }
        $env:Path=(Join-Path $cmakeRoot 'CMake/bin')+';'+(Join-Path $cmakeRoot 'Ninja')+';'+$env:Path
    }
    if($Platform -in @('Windows','Both')) {
        Prepare-Opus 'Windows'
        $windowsBuildArguments=@('--filter','sonahop-ui','tauri','build')
        if($WindowsInstaller) { $windowsBuildArguments+=@('--bundles','nsis') } else { $windowsBuildArguments+='--no-bundle' }
        Run-Native 'pnpm' $windowsBuildArguments
        $cargoTarget=if($env:CARGO_TARGET_DIR){[IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)}else{Join-Path $repoRoot 'target'}
        Copy-Item -LiteralPath (Join-Path $cargoTarget 'release/sonahop-ui.exe') -Destination (Join-Path $artifactDir 'sonahop-windows.exe') -Force
        if($WindowsInstaller) {
            $appVersion=(Get-Content -LiteralPath (Join-Path $repoRoot 'apps/sonahop-ui/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
            $installer=Get-ChildItem -LiteralPath (Join-Path $cargoTarget 'release/bundle/nsis') -Filter ('SonaHop_'+$appVersion+'_*-setup.exe') -File | Sort-Object LastWriteTime -Descending | Select-Object -First 1
            if(-not $installer) { throw 'NSIS did not produce an installer for the current application version.' }
            Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $artifactDir 'sonahop-windows-setup.exe') -Force
        }
    }
    if($Platform -in @('Android','Both')) {
        if(-not $env:JAVA_HOME -or -not $env:ANDROID_HOME -or -not $env:NDK_HOME) { throw 'Set JAVA_HOME (JDK21), ANDROID_HOME (SDK37) and NDK_HOME (30.0.16248370) before Android packaging.' }
        Prepare-Opus 'Android'
        $profile=if($AndroidRelease){'release'}else{'debug'}
        $variant=if($AndroidRelease){'Release'}else{'Debug'}
        $androidArguments=@('--filter','sonahop-ui','tauri','android','build','--apk','--split-per-abi','--target','aarch64')
        if(-not $AndroidRelease){$androidArguments+='--debug'}
        $tauriLog=Join-Path $artifactDir 'android-build.log'
        # Tauri must generate plugin Gradle dependencies and compile current Rust first.
        $savedAction=$ErrorActionPreference; $tauriExit=0
        try { $ErrorActionPreference="Continue"; & pnpm @androidArguments 2>&1 | Tee-Object -FilePath $tauriLog; $tauriExit=$LASTEXITCODE }
        finally { $ErrorActionPreference=$savedAction }
        if($tauriExit -ne 0) {
            $buildText=Get-Content -LiteralPath $tauriLog -Raw
            $symlinkOnly=$buildText -match 'Creation symbolic link is not allowed|A required privilege is not held by the client'
            if(-not $symlinkOnly -or $buildText -notmatch 'Finished .*profile') { throw "Tauri Android build failed; read $tauriLog. The symlink fallback will not reuse a library after a compilation failure." }
            $project=Join-Path $repoRoot 'apps/sonahop-ui/src-tauri/gen/android'
            $cargoTarget=if($env:CARGO_TARGET_DIR){[IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)}else{Join-Path $repoRoot 'target'}
            $library=Join-Path $cargoTarget ('aarch64-linux-android/'+$profile+'/libsonahop_ui_lib.so')
            if(-not(Test-Path -LiteralPath $library)) { throw 'Current Rust library is missing.' }
            $generatedSettings=Join-Path $project 'tauri.settings.gradle'
            if(-not(Test-Path -LiteralPath $generatedSettings)) { throw 'Tauri did not generate Android plugin settings.' }
            if((Get-Content -LiteralPath $generatedSettings -Raw) -notmatch 'sonahop-lan') { throw 'The current Android plugin is not in generated Gradle settings. Do not package an old UI.' }
            $jni=Join-Path $project 'app/src/main/jniLibs/arm64-v8a'
            New-Item -ItemType Directory -Path $jni -Force | Out-Null
            Copy-Item -LiteralPath $library -Destination (Join-Path $jni 'libsonahop_ui_lib.so') -Force
            Run-Native (Join-Path $project 'gradlew.bat') @('-p',$project,':app:clean',(':app:assembleArm64'+$variant),'-x',(':app:rustBuildArm64'+$variant),'--console=plain')
        }
        $apkDirectory=Join-Path $repoRoot ('apps/sonahop-ui/src-tauri/gen/android/app/build/outputs/apk/arm64/'+$profile)
        $apk=Join-Path $apkDirectory ('app-arm64-'+$profile+$(if($AndroidRelease){'-unsigned'}else{''})+'.apk')
        if($AndroidRelease -and -not (Test-Path -LiteralPath $apk)){ $apk=Join-Path $apkDirectory 'app-arm64-release.apk' }
        if(-not (Test-Path -LiteralPath $apk -PathType Leaf)){throw "Current Android build produced no expected APK at $apk. Read $tauriLog."}
        $apkName=if($AndroidRelease){'sonahop-android-arm64-release-unsigned.apk'}else{'sonahop-android-arm64-debug.apk'}
        Copy-Item -LiteralPath $apk -Destination (Join-Path $artifactDir $apkName) -Force
    }
    Get-ChildItem -LiteralPath $artifactDir -File | Where-Object Extension -in '.exe','.apk' |
        Get-FileHash -Algorithm SHA256 | Format-Table -AutoSize
    Write-Host "Build artifacts: $artifactDir"
} finally {
    foreach($name in $environmentNames) { [Environment]::SetEnvironmentVariable($name,$previousEnvironment[$name],'Process') }
}