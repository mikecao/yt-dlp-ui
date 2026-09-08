# Get Down

Get Down is a cross-platform desktop downloader using
[yt-dlp](https://github.com/yt-dlp/yt-dlp) for media sites and native premium
download handlers for Rapidgator, K2S, and TezFiles. Paste or drop a URL, choose
where to save it, and monitor the download without working from a terminal.

<img width="1270" height="894" alt="image" src="https://github.com/user-attachments/assets/d296da45-8352-4cd4-9585-388eab1fec5d" />

**[Download the latest release](https://github.com/mikecao/get-down/releases)**

## Features

- Download media from sites supported by yt-dlp by entering a URL or dragging and
  dropping a link onto the app.
- Run and monitor multiple downloads at the same time.
- View each download's detected filename, status, progress, speed, and size.
- Expand a download to inspect its yt-dlp console output.
- Open the original URL in your default browser.
- Cancel an active download, remove an individual item, or clear all completed and
  failed items.
- Organize downloads into independent tabs, each with its own queue, destination,
  and yt-dlp settings.
- Add, close, switch, and rename tabs. Double-click a tab name to rename it.
- Keep tabs, settings, save locations, appearance, and completed/error history
  between launches using a local SQLite database.
- Switch between light and dark themes and choose from 18 accent-color options.
- Check the installed Get Down and bundled yt-dlp versions from the About dialog.
- Avoid overwriting existing files by default.
- Save domain-specific logins with encrypted passwords in the app database and
  automatically reuse them for matching HTTPS downloads across all tabs.
- Download files from Rapidgator, K2S, and TezFiles with a verified premium account.

### Per-tab yt-dlp options

| Option | What it controls |
| --- | --- |
| Format selection | Passes a custom yt-dlp format expression with `-f` |
| Extract audio | Creates an audio-only download with `-x` |
| Audio format | Supports MP3, AAC, FLAC, WAV, Opus, M4A, and Vorbis |
| Output template | Sets the filename with `-o`. Relative templates are saved under the tab folder |
| Rate limit | Limits download speed with `--rate-limit` |
| Concurrent fragments | Downloads multiple fragments in parallel with `-N` |
| Subtitles | Downloads selected subtitle languages |
| Restricted filenames | Uses ASCII-only filenames without spaces |
| Browser cookies | Reads cookies from Chrome, Firefox, Edge, Safari, Opera, or Brave |
| Custom arguments | Appends additional yt-dlp arguments, including quoted values |

## Supported platforms

The release workflow builds packages for:

- Windows x64 (`.msi`)
- macOS on Apple Silicon and Intel (`.dmg` / `.app`)
- Linux x64 (`.AppImage`)

Get Down bundles a platform-specific yt-dlp executable, so a separate yt-dlp
installation is not required. Some format conversion, audio extraction, and
video/audio merging operations still require
[FFmpeg](https://ffmpeg.org/) to be installed and available on your `PATH`.

## Usage

1. Open Get Down and click **Select** beside **Save to** to choose a destination.
2. Paste a URL into **Enter URL** and press <kbd>Enter</kbd> or click
   **Download**. You can also drag a link onto the application window.
3. Follow the download in the table. Click the terminal icon on an item to view
   detailed yt-dlp output.
4. Use the **Settings** view when you need a specific format, audio extraction,
   subtitles, browser cookies, rate limiting, or custom yt-dlp arguments.

Downloads start immediately and use the settings of their current tab. The default
output filename is `%(title)s.%(ext)s` (configurable with `-o` in Settings), and
an existing file with the same name will not be overwritten.

### Saved logins

Click the gear icon to open **Settings → Saved Logins → Add login**, enter a domain (or paste a website
URL), username, and password, then click **Save login**. Logins are shared across
tabs and persist between launches. Use **Edit** to update an account (leave the
password blank to keep it), or **Remove** to delete it.

Login matching automatically uses the most specific saved domain. For example,
`members.example.com` uses its own login when saved, otherwise it falls back to
`example.com`. Parent-domain logins apply to their subdomains without a separate
setting. A subdomain login never applies to its parent or siblings. Saved
credentials for yt-dlp are only applied to HTTPS URLs and take precedence over
username/password options in custom arguments.

Saved credentials are encrypted with AES-256-GCM in the app's SQLite database.
Each write uses a fresh random nonce, and encryption binds the credential to its
domain. The domain and displayed username remain readable metadata. A single random encryption key is stored in Windows Credential Manager,
macOS Keychain, or the Linux Secret Service keyring. Linux requires a running,
unlocked Secret Service such as GNOME Keyring or KWallet.

Existing per-site OS-store logins migrate automatically when accessed. The old
entry is removed only after the encrypted database copy is committed. Interrupted
cleanup is retried, and inaccessible old credentials remain listed with a warning.
Saving a login updates its metadata and encrypted credentials in one database
operation; deleting it removes the database record.

A database backup alone cannot decrypt passwords on another computer: the original
OS-protected key is also required. If that key disappears while encrypted records
exist, Get Down reports an error instead of silently creating a replacement.

For yt-dlp sites, credentials are decrypted when a download starts and passed to
the yt-dlp process; known credential values are redacted from displayed and saved
output. Local process inspection may expose them while yt-dlp runs. For the three
premium hosts below, passwords and session tokens stay in the native application
and are sent only to the appropriate host's HTTPS API; they are not passed to
yt-dlp or included in logs.

Other saved domains use yt-dlp's username/password authentication and do not create
a browser session. Site support varies; for sites requiring a browser login, use the
existing **Cookies from Browser** setting instead. See
[yt-dlp authentication options](https://github.com/yt-dlp/yt-dlp#authentication-options).

### Premium file hosts

In **Settings → Saved Logins**, click **Add login**, enter `rapidgator.net`,
`k2s.cc`, or `tezfiles.com`, then save the account's email and password. Paste a file
link into Downloads as usual. The app checks premium status and remaining
traffic before requesting a download link. Missing, free, or expired accounts
cannot start a download, and there is no free-download fallback.

The `www` variants of these domains are recognized automatically; `keep2share.cc`
is also recognized as K2S. K2S and TezFiles credentials are kept separate.
HTTP page links are accepted, but all account and file requests use HTTPS.
Uploadgig is not included in these integrations.

Premium files use the original filename (with unsafe filename characters replaced),
the tab's save folder, and its rate limit. Other yt-dlp settings, including output
templates, format conversion, browser cookies, and custom arguments, do not apply.
Progress, speed, cancellation, and completed/error history use the normal download
list. Existing destination files are never overwritten. Failed or cancelled
transfers keep a uniquely named `.part` file; retrying starts a fresh transfer.

API sessions are reused in memory while the app runs and cleared when logins
change. K2S and TezFiles login CAPTCHAs can be completed in your browser: open
the verification page from the download row, then paste its code into Get Down.
Verification expires after ten minutes. Other login challenges (including 2FA)
and automatic resume are not implemented.
Some files may require a particular publisher subscription in addition to a
premium account. Protocol references:
[Rapidgator API](https://rapidgator.net/article/api/user) and
[K2S/TezFiles API](https://keep2share.github.io/api/).

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) LTS
- [pnpm](https://pnpm.io/) 10.33.2
- The stable [Rust toolchain](https://www.rust-lang.org/tools/install)
- The [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/)
  for your operating system

### Set up the project

```shell
git clone https://github.com/mikecao/get-down.git
cd get-down
pnpm install
pnpm download-binaries
```

`pnpm download-binaries` fetches the latest yt-dlp executable for the current
platform and places it in `src-tauri/binaries/` under the target-specific filename
expected by Tauri.

### Run in development

```shell
pnpm dev
```

This starts the Vite frontend and opens it in the Tauri desktop shell.

### Create a production build

```shell
pnpm build
```

The native executable is written below `src-tauri/target/release/`, and packaged
installers are written below `src-tauri/target/release/bundle/`.

### Available scripts

| Command | Description |
| --- | --- |
| `pnpm dev` | Run the complete Tauri application in development mode |
| `pnpm build` | Build the frontend and native application bundles |
| `pnpm vite-dev` | Run only the Vite development server |
| `pnpm vite-build` | Build only the web frontend |
| `pnpm preview` | Preview the built web frontend |
| `pnpm download-binaries` | Download the latest yt-dlp sidecar for this platform |
| `pnpm lint` | Lint the project with Biome |
| `pnpm format` | Format the project with Biome |
| `pnpm check` | Run all configured Biome checks |

## Project structure

```text
src/                         React application
  components/                Download, settings, tab, and shared UI components
  lib/                       State, SQLite, theming, and yt-dlp argument helpers
src-tauri/                   Tauri/Rust desktop application
  binaries/                  Platform-specific yt-dlp sidecars
  capabilities/default.json Tauri permissions
  src/main.rs                Native application entry point and SQLite migration
scripts/download-binaries.mjs
                             yt-dlp sidecar download script
```

The frontend is built with React 19, TypeScript, Vite, Tailwind CSS, Zustand, and
Base UI. Tauri v2 provides the native shell, file picker, process management, and
SQLite storage. Each download runs through the bundled yt-dlp sidecar.

## Notes

- Site support and behavior are determined by yt-dlp. See its
  [supported-sites list](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md).
- Browser-cookie downloads use the selected browser's local cookie store and may
  require the browser to be closed or otherwise accessible to yt-dlp.
- DRM-protected media is not supported. Only download content you are authorized
  to access and save.
- Active or cancelled downloads are not restored after restarting the app;
  completed and failed entries are retained.

## License

Get Down is released under the [MIT License](LICENSE).
