# Releasing GuitarCraft

GuitarCraft follows the release pattern of storytold's crafting apps (SoundCraft, DesignCraft,
PhotoCraft): the version lives in one place, and every push to the `release` branch runs
`.github/workflows/release.yml`, which builds the Windows installer and creates or updates a
**draft** GitHub Release named `GuitarCraft v<version>`. Nobody sees a draft until a maintainer
publishes it. For now only Windows x64 is packaged.

User-facing names say **GuitarCraft**. Files, binaries and ids stay lowercase
(`guitarcraft-<version>-windows-x64.msi`).

## The version

The version lives in exactly one place: `version` under `[package]` in the root `Cargo.toml`.
Everything else reads it from there:

- the app (the home screen shows *Version …*, through `CARGO_PKG_VERSION`),
- `guitarcraft.exe`'s file and product version (embedded by `build.rs`),
- the installer and zip file names, and the MSI version (`package.ps1`).

Change it with the xtask, never by hand, so `Cargo.lock` follows:

```sh
cargo xtask version                 # prints the current version, e.g. 0.1.0
cargo xtask version set 0.2.0       # or 0.2.0-rc.1; updates Cargo.toml and Cargo.lock
```

**When to bump:** once per release, on a branch, before packaging it. Commit the bump
(`Cargo.toml` + `Cargo.lock`) through a pull request like any other change.

- `0.1.0 → 0.1.1` for a fix-only release,
- `0.1.0 → 0.2.0` for a release with new features (while GuitarCraft is below 1.0),
- `0.2.0-rc.1`, `0.2.0-rc.2`, … for test builds before 0.2.0. Each one installs over the last.

The MSI version is the numeric `X.Y.Z` (Windows Installer allows up to 255.255.65535, and the
xtask refuses anything bigger). A pre-release tag is dropped there, so `0.2.0-rc.1` and
`0.2.0-rc.2` are both MSI `0.2.0`; the installer allows same-version upgrades so they replace
each other, and then `0.2.0` replaces them. An older version never installs over a newer one.

## Cutting a release

1. **Bump the version** (above) and merge it to `main`.
2. **Merge `main` into `release`** (or fast-forward it) and push. The workflow starts by itself:

   ```sh
   git push origin main:release
   ```

3. **Wait for the draft.** After about 15 minutes the Releases page has a draft
   `GuitarCraft v<version>`, tagged `v<version>` on the pushed commit, with notes generated from
   the merged PRs and these files:

   | File | What it is |
   |---|---|
   | `guitarcraft-<v>-windows-x64.msi` | the installer |
   | `guitarcraft-<v>-windows-x64-portable.zip` | `guitarcraft.exe`, `README.md` and `LICENSE`, to run without installing |
   | `SHA256SUMS.txt` | checksums of both |

4. **Check it.** Download the MSI, install it, open a song from Explorer, uninstall it again.
   A `::warning::` in the job summary means the files are unsigned (see Signing).
5. **Publish** the draft in the GitHub UI. Publishing creates the `v<version>` tag. Versions
   with a pre-release tag (`-rc.1`) are marked as pre-releases.

Pushing to `release` again before you publish rebuilds the same draft and replaces its files.
After the draft is published, the workflow refuses to touch that version again, so bump it first.

**Test runs:** *Actions → Release → Run workflow* builds the packages by hand from any branch.
The optional `version` input (such as `0.2.0-rc.1`) overrides `Cargo.toml` for that run only;
the job applies it with `cargo xtask version set` before building, so the exe reports it too.
Only a run on `release` drafts a release; on other branches the files are left as the run's
artifacts.

**Locally**, the same packages can be built on Windows into `dist/release/`:

```powershell
powershell -ExecutionPolicy Bypass -File packaging/windows/package.ps1
```

## Secrets

Signing secrets (see Signing) belong in the repository's **`release` environment** (*Settings →
Environments → release*), restricted to the `release` branch. Only the Windows job and the
draft-release job use that environment. GitHub creates the environment on the first run if it
doesn't exist; add the branch restriction there. `GITHUB_TOKEN` creates the release, and only the
draft-release job gets `contents: write`.

## What the build does

`packaging/windows/package.ps1`:

- builds `guitarcraft.exe` with `-C target-feature=+crt-static`, so neither the MSI nor the zip
  needs the Visual C++ redistributable;
- sets `GUITARCRAFT_REQUIRE_WINRES=1` so `build.rs` fails, rather than warns, if the icon and
  version info can't be embedded;
- checks the exe's PE header: x64, and a Windows GUI app (no console window opens with it);
- checks the exe reports the version being packaged;
- signs the exe and the MSI with `sign.ps1` (see Signing);
- builds the MSI from `guitarcraft.wxs` with WiX v5.

Needs: Rust, the Windows SDK (installed with the Visual Studio C++ build tools), and WiX v5:

```powershell
dotnet tool install --global wix --version 5.0.2
```

## The installer

`packaging/windows/guitarcraft.wxs` (WiX v5), with the install-scope logic in
`install-policy.wxi`, both adapted from SoundCraft's:

- **Who it's for:** the first page asks *Only for me* (the default, no administrator needed,
  into `%LOCALAPPDATA%\Programs\CodeGator\GuitarCraft`) or *Everyone on this computer* (asks
  an administrator, into `Program Files\CodeGator\GuitarCraft`). An update keeps the folder
  the first install chose. A silent install for everyone passes `ALLUSERS=1` to `msiexec`.
- **Start menu** shortcut, and the *Programs and Features* entry with the icon and links to
  the GitHub page.
- **Files:** GuitarCraft becomes the default app for `.chart` chord charts. For `.gp`, `.gp5`,
  `.gpx`, `.gp4`, `.gp3`, `.mid`, `.midi` and old `.gcchart` charts it's listed under
  *Open with*, but never takes them over, so an installed Guitar Pro keeps its files.
- **Win+R** `guitarcraft` starts it (App Paths).
- Settings in `%APPDATA%\GuitarCraft` belong to the user and are left alone on uninstall.

The upgrade code `82E6056C-F62C-41C2-A2C2-DE5F329175D9` is fixed forever. Changing it would make
Windows treat the next release as a different product and install it beside the old one.

## Signing

Without a code-signing certificate the installer works, but Windows SmartScreen warns that the
publisher is unknown, and the user has to click *More info → Run anyway*. `sign.ps1` signs
when it finds one of:

- `WINDOWS_CERTIFICATE` (a base64 `.pfx`) and `WINDOWS_CERTIFICATE_PASSWORD`, or
- Azure Trusted Signing: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`,
  `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE`.

With neither it prints a warning and leaves the files unsigned.

## Icons

`assets/app-icon/icon.svg` is the canonical icon, a **placeholder**: a guitar pick over six
strings. `cargo xtask icons` renders it into `guitarcraft-1024.png`, `guitarcraft-256.png`
(the window and taskbar icon, built into the app) and `guitarcraft.ico` (9 sizes, 16 to
256 px, embedded in the exe and the installer). The outputs are committed, so a build never
needs to regenerate them.

## Not done yet

Compared with the storytold apps, these are left for later:

- x86 and ARM64 Windows builds, macOS (`.dmg`) and Linux (AppImage, `.deb`, `.rpm`);
- `check-install-policy.ps1`, which checks the compiled MSI's scope rules without installing.
