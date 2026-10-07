# local-sights

A local desktop viewer for Amazon CloudWatch Logs, written in Rust (Tauri 2)
with a React + TypeScript screen. It calls only the read-only
`DescribeLogGroups`, `DescribeLogStreams` and `GetLogEvents` APIs and sends
no telemetry or crash reports anywhere.

> Status: units U1 to U6. You choose a profile and a region, pick a log
> group from the list (with a filter), enter a time range in local time or
> UTC (switch in the top bar) and press
> **Fetch**: every stream of the group that matters for the range is read,
> one after another, and the events are listed in time order as they arrive
> ("time / stream / message"). Throttling and network errors are retried;
> streams that still fail are listed separately while the rest is shown.
> The list draws only the visible rows, so a million events scroll smoothly.
> The log filter narrows the fetched events to those whose message contains
> a text (ignoring case), on your machine, without calling AWS again.
> An optional disk cache (off by default, **[\*]** in the top bar) keeps
> fully successful fetches on your machine and shows a range it already
> holds without calling AWS.

Required IAM permissions: `logs:DescribeLogGroups`, `logs:DescribeLogStreams`
and `logs:GetLogEvents`.

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

Choose a profile ("Default settings (left to the SDK)" uses the SDK default
chain) and a region (the profile's default region is filled in when it has
one), pick a log group in the left pane, then enter the times
(`yyyy-mm-dd hh:mm:ss` in the time zone chosen in the top bar; the end
second is included up to its last millisecond) and press **Fetch** or Enter.
The time zone is Local (the OS setting, with its daylight saving rules) at
every launch and can be switched to UTC at any time, also while fetching;
the choice is not saved. Switching keeps the entered instants (the texts are
rewritten) and re-shows the log times without fetching again. A local time
skipped by a daylight saving change is refused with its own reason; a local
time that occurs twice means the earlier one. If the OS time zone cannot be
read, Local uses UTC and a note with the zone name goes to standard error. There is no stream to type: the
streams whose events may fall in the range are found with
`DescribeLogStreams` (newest last event first, stopping at streams whose last
event is more than an hour before the start). The profile list comes from
`~/.aws/config` and `~/.aws/credentials` (or `AWS_CONFIG_FILE` /
`AWS_SHARED_CREDENTIALS_FILE`); only section names and `region` values are
read, never credentials.

## Manual GUI check (walking-skeleton checkpoint)

Run these on your own machine with your own AWS credentials (`npm run tauri dev`):

- [ ] The app starts and shows the profile and region selectors, the log group pane, the selected log group ("Not selected"), the time inputs, the **Fetch** button and the reasons why Fetch is unavailable.
- [ ] With valid conditions, pressing Enter in any input starts the fetch (same as clicking **Fetch**).
- [ ] While fetching, every input and **Fetch** are disabled and the status line says it is fetching.
- [ ] The fetched rows appear with the time to the millisecond (`yyyy-mm-dd hh:mm:ss.mmm`) in the chosen time zone (Local at launch, U4).
- [ ] A long message, or one with line breaks, is shown on one line and cut with an ellipsis (…) at the edge of the column.
- [ ] The status line shows the event count, and for a range with no logs says explicitly that there are 0 events.
- [ ] On an error (for example a misspelled log group, or a profile without a default region), the status line shows the error kind and the safe detail, which contains no secret or access key ID; rows fetched before the error stay visible.
- [ ] After a fetch finishes, the inputs are enabled again and fetching again replaces the previous rows.

Connection and log groups (U2):

- [ ] Profile and region start unselected and the pane asks for a profile. The profile list starts with "Default settings (left to the SDK)", followed by your profiles in name order.
- [ ] Choosing a profile with a default region fills in the region and the log group list loads ("Loading…" while pages arrive; groups can be selected meanwhile).
- [ ] Choosing a profile without a default region leaves the region unselected and the pane asks for a region; choosing one loads the list.
- [ ] The filter narrows the list case-insensitively (e.g. `myfunc` finds `/aws/lambda/MyFunction`); a filter with no match says so; an account without log groups says there are none.
- [ ] Selecting a group (click, or arrow keys then Enter/Space) shows its name next to the time inputs, even when the filter hides it. **Reload** re-reads the list and keeps the selection and the shown logs.
- [ ] With logs shown, changing the profile or region asks "The logs shown will be cleared. Change the connection?" with focus on **Cancel**; **Cancel** or Escape keeps everything, **Change** clears the logs, count and error, deselects the log group and loads the new list.
- [ ] While fetching, the profile, region, log group list and **Reload** are disabled.
- [ ] With a profile that lacks permission (or an expired SSO login), the pane shows that the list is incomplete with the error kind and the safe detail, and the app keeps running; you can choose another profile or region.
- [ ] If `~/.aws/credentials` (or config) exists but cannot be read, a notice near the profile selector names the file kind only (no path, no content).

Whole-group fetch (U3):

- [ ] There is no stream input; **Fetch** is available once a log group is selected and the times are valid.
- [ ] While the streams are listed, the status line says "Listing streams…" with the number selected so far; then "Fetching… n/m streams" with the events so far.
- [ ] For a group with several active streams, the rows of all streams are mixed in time order (equal times: by stream name, then the order within the stream) with the stream name in its own column.
- [ ] With close to a million events, scrolling (mouse, arrow keys, Page Up, Page Down, Home, End) stays responsive and reaches the first and last rows.
- [ ] While a fetch adds rows, a list scrolled down keeps the row at the top where it is; a list at the very top stays at the top.
- [ ] When some streams fail (for example with a role that may not read one stream), the status line shows "Failed in N streams"; pressing it (or Tab to it and Enter/Space) opens the list of stream names with the error kind and safe detail, focus on **Close**; **Close** or Escape closes it, and fetching again empties it.
- [ ] Streams without events or without times: with a group that has an empty stream and a start later than its old streams, `fetch_check` reports the empty stream among the streams fetched (`streams:` on standard error). This checks the assumption about where such streams appear in the listing (rules.md, BR1.2); if it does not hold, note it for the next unit.

Time zones (U4):

- [ ] At launch the top bar shows the time zone switch with **Local** selected; the time inputs read "Start (Local)" / "End (Local)" and the time column "Time (Local)".
- [ ] With a fetch shown, switching to **UTC** (click, or Tab to the switch and use the arrow keys or Space) changes every row's time and the column header to UTC at once, without "Fetching…" and without the rows moving; switching back restores the local times.
- [ ] A start entered in local time (for example `2024-03-01 10:00:00` with a UTC+9 OS zone) becomes the same instant in UTC (`2024-03-01 01:00:00`) when switching, and back again; a malformed input keeps its text and its reason across switches.
- [ ] The switch stays usable while fetching and while the connection-change dialog is open; switching then changes neither the fetch nor the dialog.
- [ ] In a time zone with daylight saving time (for example set the OS zone to America/New_York and restart the app), entering a skipped local time (`2024-03-10 02:30:00`) disables **Fetch** with "The start does not exist because of the daylight saving time change."; switching to UTC makes it a valid UTC time and the reason disappears. Rows on both sides of a daylight saving change show the offset in force at their own time.

Log filter (U5):

- [ ] The condition area has a "Filter logs" field (Tab reaches it). Typing `error` and pausing for about 0.3 s shows only the rows whose message contains it in any case (for example `Error: timeout`), including a match on the second line of a message; a stream name that contains the text does not make a row match.
- [ ] The status line shows "Filtered: N of M events"; a text that matches nothing shows "Filtered: 0 of M events, no logs match."; clearing the field shows every row again and removes the filtered count. Changing the text, or clearing it, goes back to the top of the list.
- [ ] Pressing Enter in the filter field does not start a fetch.
- [ ] While fetching, the filter field stays usable; with a text in it, only the matching rows of the arriving pages are added and both numbers of "Filtered: N of M events" grow. A list scrolled down keeps its top row in place while rows are added.
- [ ] With a text in the field, pressing **Fetch** again keeps the text; the new rows are filtered with it from the start.
- [ ] While the connection-change dialog is open, the filter field cannot be used.
- [ ] With close to a million events held, changing the filter shows "Filtering…" next to the count while matching rows appear little by little; scrolling and typing keep responding within about a second, and the result is complete ("Filtering…" disappears) within 100 seconds.
- [ ] Pressing Enter in the filter field filters at once, without waiting 0.3 s (U6:BR5.4).

Disk cache (U6):

- [ ] **[\*]** in the top bar opens the settings dialog with focus on the checkbox; it shows the cache location (on macOS under `~/Library/Caches/dev.local-sights.app/log-cache`) and the warning that logs may contain confidential information, checked or not. Tab cycles inside the dialog, Space toggles the checkbox, Escape or **Cancel** closes it without saving and focus returns to **[\*]**. **[\*]** cannot be pressed while fetching or while the connection-change dialog is open.
- [ ] Enable the cache and **Save**; restart the app and open the dialog again: it is still enabled (the settings file is under `~/Library/Application Support/dev.local-sights.app/`, owner-only, holding only the setting). The cache folder and its files are owner-only (`ls -ld` shows `drwx------`, files `-rw-------`).
- [ ] Fetch a range that ended more than five minutes ago: the status line says "Saving to cache" before the fetch ends. Fetch the same profile, region, log group and range again: the rows appear with "Shown from cache (AWS was not called)" and no stream counts, and no AWS call is made (for example, CloudTrail or a proxy shows none). The log filter works on these rows too.
- [ ] Fetch a range whose end is within five minutes of now: fetching it again goes to AWS (the newest five minutes are never cached); a part of it older than five minutes is served from the cache.
- [ ] Widen a cached range: it goes to AWS, and afterwards the wider range is served from the cache.
- [ ] Damage a cache file (for example cut its last line with `truncate -s -10 <file>`): the next fetch of that range says "The cache could not be read, so the logs were fetched again" and the file is replaced by a good one.
- [ ] **Clear cache** removes the cache files at once ("The cache was cleared.") and **Cancel** does not bring them back; unchecking the box and **Save** also removes them and closes the dialog.
- [ ] With close to a million events, note how long "Saving to cache" lasts (the fetch ends only after the write; the design sets no bound, NFR2), and that the screen keeps scrolling meanwhile.

## Check against real AWS from the terminal

`fetch_check` uses the same validation and fetch flow as the app and reads
the whole log group. Its `--start` and `--end` are always UTC and it prints
UTC times; the time zone switch belongs to the screen only (U4:BR3.5), and
so does the log filter (U5:BR3.5). Run it only on your own machine with your own
credentials; tests and CI never run it.

```bash
cargo run -p local-sights-core --example fetch_check -- \
  --profile dev --log-group /aws/lambda/my-function \
  --start '2024-01-02 03:00:00' --end '2024-01-02 03:59:59'
```

Events go to standard output in time order
(`UTC time<TAB>stream name<TAB>message`); the event, stream and failed-stream
counts, then each failed stream (or an incomplete stream listing) with its
failure kind and safe detail, go to standard error. Exit code 0 means
success, 1 that something failed, 2 invalid arguments.

## Tests and checks

Automated tests never connect to AWS: the AWS SDK sits behind the
`CloudWatchLogsGateway` trait and tests use a fake. They do not depend on
the OS time zone either: the local zone is passed in as a fixed IANA zone
(`America/New_York`, `Asia/Tokyo`, `UTC`).

```bash
cargo test --workspace          # Rust library and integration tests
cargo test -p local-sights-core --release --lib -- timeline:: --ignored  # 1M-event speed check
cargo test -p local-sights-core --release --lib -- filter:: log_view:: --ignored  # filter speed (NFR1, NFR2)
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
