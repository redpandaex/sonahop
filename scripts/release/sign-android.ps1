[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$InputApk,
    [Parameter(Mandatory)][string]$OutputApk,
    [Parameter(Mandatory)][string]$Version
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
foreach($audioSecret in @('ANDROID_KEYSTORE_PATH','ANDROID_KEYSTORE_PASSWORD','ANDROID_KEY_ALIAS','ANDROID_KEY_PASSWORD')) {
    if([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($audioSecret,'Process'))) {
        throw "Configure $audioSecret before signing. No debug signing fallback is allowed."
    }
}
if(-not (Test-Path -LiteralPath $env:ANDROID_KEYSTORE_PATH -PathType Leaf)) {throw 'Android keystore file is missing.'}
$audioInput=[IO.Path]::GetFullPath($InputApk)
$audioOutput=[IO.Path]::GetFullPath($OutputApk)
if($audioInput -eq $audioOutput) {throw 'Signed output must be separate from the unsigned input.'}
$audioTools=Join-Path $env:ANDROID_HOME 'build-tools/36.0.0'
$audioSigner=Join-Path $audioTools 'apksigner.bat'
$audioAligner=Join-Path $audioTools 'zipalign.exe'
$audioAapt=Join-Path $audioTools 'aapt2.exe'
foreach($audioTool in @($audioSigner,$audioAligner,$audioAapt)) {
    if(-not (Test-Path -LiteralPath $audioTool -PathType Leaf)) {throw 'Install Android build-tools 36.0.0.'}
}
$audioBadging=& $audioAapt dump badging $audioInput
if($LASTEXITCODE -ne 0) {throw 'Cannot inspect unsigned APK.'}
if($audioBadging -match '^application-debuggable') {throw 'Refusing to publish a debuggable APK.'}
if(-not ($audioBadging -match ("name='io.github.redpandaex.sonahop'.*versionName='"+[regex]::Escape($Version)+"'"))) {throw 'APK identity/version does not match the release.'}
$audioOutputDirectory=[IO.Path]::GetDirectoryName($audioOutput)
New-Item -ItemType Directory -Path $audioOutputDirectory -Force | Out-Null
$audioAligned=Join-Path ([IO.Path]::GetTempPath()) ('sonahop-aligned-'+[guid]::NewGuid().ToString('N')+'.apk')
try {
    & $audioAligner -P 16 -f 4 $audioInput $audioAligned
    if($LASTEXITCODE -ne 0) {throw 'APK alignment failed.'}
    # Passwords are passed by environment name, never placed in a command argument or log.
    & $audioSigner sign --ks $env:ANDROID_KEYSTORE_PATH --ks-key-alias $env:ANDROID_KEY_ALIAS --ks-pass env:ANDROID_KEYSTORE_PASSWORD --key-pass env:ANDROID_KEY_PASSWORD --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true --out $audioOutput $audioAligned
    if($LASTEXITCODE -ne 0) {throw 'APK release signing failed.'}
    $audioVerification=& $audioSigner verify --verbose --print-certs $audioOutput
    if($LASTEXITCODE -ne 0) {throw 'Signed APK verification failed.'}
    if(-not ($audioVerification -match 'APK Signature Scheme v2\): true') -or -not ($audioVerification -match 'APK Signature Scheme v3\): true')) {throw 'Release APK must verify with v2 and v3 signatures.'}
    if($audioVerification -match '^Signer #1 certificate DN:.*CN=Android Debug') {throw 'Use a dedicated release keystore, not the Android debug key.'}
    $audioCertLine=$audioVerification | Where-Object {$_ -match '^Signer #1 certificate SHA-256 digest:'} | Select-Object -First 1
    if(-not $audioCertLine) {throw 'APK signing certificate fingerprint is missing.'}
    $audioFingerprint=($audioCertLine -split ': ',2)[1].Trim()
    $audioSignatureJson=[pscustomobject]@{verified=$true;v2=$true;v3=$true;certificateSha256=$audioFingerprint} | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $audioOutputDirectory 'android-signature.json'),$audioSignatureJson,[Text.UTF8Encoding]::new($false))
    Write-Host ('Release APK signed and verified; certificate SHA256: '+$audioFingerprint)
} finally {
    if(Test-Path -LiteralPath $audioAligned -PathType Leaf) {Remove-Item -LiteralPath $audioAligned -Force}
}
