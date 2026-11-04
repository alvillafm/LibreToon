# Package checks and optional Windows signing

The existing workflow remains **manual (`workflow_dispatch`) only**. It does not publish Releases. There is no change to LibreToon's MIT license, and FFmpeg remains a separate optional user download.

## Checks before download artifacts are uploaded

- Windows: check application/installer product, publisher and package version; verify signatures when requested; verify the portable contains the same executable; check package SHA-256 values; update Microsoft Defender intelligence and scan the unpacked payload and final packages.
- macOS/Linux: update ClamAV definitions and scan final archives and the unpacked native application. Definitions/logs stay outside all payloads.
- A missing/stopped scanner, failed definition update, definitions older than 48 hours, detection, scan error, checksum mismatch or invalid signature blocks the package upload. Logs and JSON reports are available as separate security artifacts, including on failure.
- Windows scans use custom scan mode with `DisableRemediation`: detections cannot become an exit-code-zero result because a file was silently removed. This scan option ignores file exclusions and includes archives; it does not disable antivirus protection.
- A passing report records the particular engine, definitions and hashes. It does **not** certify a malware-free program or guarantee acceptance by Chrome, SmartScreen or Gatekeeper.

The Windows portable and installer have separate artifact links in the run summary. GitHub still wraps downloaded artifacts in ZIPs. Their names explicitly say `unsigned` or `signed`; the latter is available only after timestamped Authenticode verification.

## Optional Windows signing

Select `windows_signing: none` while no verified signing identity is configured. This builds unsigned packages with security checks and explicitly reports that state. No self-signed certificate is manufactured and no account or paid service is created automatically.

For public-trust signing, provision your own **Azure Artifact Signing** account, verified identity and public-trust certificate profile. Create an Entra application/service principal, grant **Artifact Signing Certificate Profile Signer** on the intended profile and configure GitHub OIDC federation for `repo:alvillafm/LibreToon:ref:refs/heads/main`. Configure these repository **variables**:

- `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`
- `WINDOWS_SIGNING_ENDPOINT` (the account's regional HTTPS `*.codesigning.azure.net` endpoint)
- `WINDOWS_SIGNING_ACCOUNT`, `WINDOWS_SIGNING_PROFILE`

Then manually run the workflow on `main`, selecting `windows_signing: azure`. The integration uses the verified upstream Azure/login v3 commit and ArtifactSigning PowerShell module 0.1.8. It uses Azure CLI/OIDC authentication without storing/exporting certificate private keys in the repository. Inno Setup invokes the same signing adapter for its uninstaller, temporary self-copies and installer; the application is signed before the portable ZIP is created. SHA-256 manifests are generated after signing.

Signing remains **pending until these external prerequisites are configured and a signed CI execution passes**. This change does not add Apple Developer ID signing or notarization: existing macOS ad-hoc signing remains unchanged.

## Validation and blocked downloads

Local tests: `python scripts/packaging/test_security.py` and `python scripts/packaging/test_native.py`. They simulate scan success, detections, unavailable scanners, outdated definitions and failures without changing local antivirus settings or contacting signing services. Windows executable metadata is also verified against the local release build. The real hosted antivirus/signing integrations must be checked in the next manually requested CI execution.

If Chrome continues blocking a hash despite clean independent scans, retain the exact binary, SHA-256 and security report, then request a review from the security provider. Do not disable browser protection to label a package as safe. No files are automatically submitted to Google, VirusTotal or other analysis websites.

Sources: [Microsoft Defender command-line behavior](https://learn.microsoft.com/en-us/defender-endpoint/command-line-arguments-microsoft-defender-antivirus), [Azure signing integrations](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations), [official signing action and module parameters](https://github.com/Azure/artifact-signing-action/blob/v2/action.yml), [Inno Setup signing](https://jrsoftware.org/ishelp/topic_setup_signtool.htm).
