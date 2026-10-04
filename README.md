# local-sights

A local desktop viewer for Amazon CloudWatch Logs, written in Rust (Tauri 2)
with a React + TypeScript screen. It reads logs with the read-only
`GetLogEvents` API only and sends no telemetry or crash reports anywhere.

> Status: walking skeleton (unit U1). You type a profile, a log group, a log
> stream and a UTC time range, press **Fetch**, and every event of that one
> stream in the range is listed. Choosing from lists, multiple streams,
> retries, time zones, filtering and caching come in later units.

## Layout

| Path                                               | What it is                                                                                                           |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `crates/local-sights-core/`                        | GUI-independent library: validation, time range, paging, gateway (AWS SDK behind a trait), fetch flow, session state |
| `crates/local-sights-core/examples/fetch_check.rs` | Developer-only command-line check against real AWS                                                                   |
| `src-tauri/`                                       | Tauri app (`local-sights`): commands and events only                                                                 |
| `src/`                                             | Screen (Vite + React + TypeScript)                                                                                   |

## Prerequisites

- Rust stable (1.85 or later) via [rustup](https://rustup.rs/)
- Node.js 22 and npm
- Tauri 2 system prerequisites for your OS: see
  <https://v2.tauri.app/start/prerequisites/>. On Debian/Ubuntu:

  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev build-essential pkg-config libssl-dev
  ```

- An AWS CLI-style configuration (`~/.aws/config`) with a default region for
  the profile you use. Authentication (SSO, access keys, AssumeRole,
  environment variables) is left to the AWS SDK; for SSO profiles run
  `aws sso login --profile <name>` first. The app never reads, stores or logs
  your credentials.

## Run the app

```bash
npm ci
npm run tauri dev      # development window with hot reload
npm run tauri build    # release bundle under target/release/bundle/
```

Or install the binary from source (builds the screen first):

```bash
npm ci && npm run build
cargo install --path src-tauri --features custom-protocol
```

Enter the conditions (times are UTC, `yyyy-mm-dd hh:mm:ss`; the end second is
included up to its last millisecond) and press **Fetch** or Enter. Leave the
profile blank to use the SDK default profile.

## Manual GUI check (walking-skeleton checkpoint)

Run these on your own machine with your own AWS credentials (`npm run tauri dev`):

- [ ] The app starts and shows the five inputs, the **Fetch** button and the reasons why Fetch is unavailable.
- [ ] With valid conditions, pressing Enter in any input starts the fetch (same as clicking **Fetch**).
- [ ] While fetching, every input and **Fetch** are disabled and the status line says it is fetching.
- [ ] The fetched rows appear with the time in UTC to the millisecond (`yyyy-mm-dd hh:mm:ss.mmm`).
- [ ] A long message, or one with line breaks, is shown on one line and cut with an ellipsis (…) at the edge of the column.
- [ ] The status line shows the event count, and for a range with no logs says explicitly that there are 0 events.
- [ ] On an error (for example a misspelled log group, or a profile without a default region), the status line shows the error kind and the safe detail, which contains no secret or access key ID; rows fetched before the error stay visible.
- [ ] After a fetch finishes, the inputs are enabled again and fetching again replaces the previous rows.

## Check against real AWS from the terminal

`fetch_check` uses the same validation and fetch flow as the app. Run it only
on your own machine with your own credentials; tests and CI never run it.

```bash
cargo run -p local-sights-core --example fetch_check -- \
  --profile dev --log-group /aws/lambda/my-function \
  --stream '2024/01/02/[$LATEST]0123456789abcdef' \
  --start '2024-01-02 03:00:00' --end '2024-01-02 03:59:59'
```

Events go to standard output (`UTC time<TAB>message`); the event and page
counts, or the failure kind with its safe detail, go to standard error. Exit
code 0 means success, 1 a failed fetch, 2 invalid arguments.

## Tests and checks

Automated tests never connect to AWS: the AWS SDK sits behind the
`CloudWatchLogsGateway` trait and tests use a fake.

```bash
cargo test --workspace          # Rust library and integration tests
npm test                        # screen tests (Vitest + React Testing Library)
cargo fmt --check
cargo clippy --workspace
npm run typecheck && npm run lint && npm run format:check
npm audit
```

## Secrets

Never commit AWS credentials, access key IDs, session tokens or SSO tokens.
Errors and diagnostics show only safe details (error kind, API name, request
ID, profile name, log group, log stream); anything that looks like an access
key ID, secret access key or session token is masked as `[REDACTED]`.

## License

Apache License 2.0. See [LICENSE](LICENSE).
