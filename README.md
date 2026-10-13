# Local Sights

A local desktop viewer for Amazon CloudWatch Logs (the app is named "Local
Sights"; the package, crate and binary are `local-sights`), written in Rust
(Tauri 2) with a React + TypeScript screen. It calls only the read-only
`DescribeLogGroups`, `DescribeLogStreams` and `GetLogEvents` APIs and sends
no telemetry or crash reports anywhere.

> Status: units U1 to U7, plus the console-style screen. The screen is drawn
> with the [Cloudscape Design System](https://cloudscape.design/) (the
> components of the AWS Management Console) and follows the OS light or dark
> mode. You choose a profile and a region in the black top navigation, pick a
> log group from the table on the left (with a search field), set the start
> and end as a date (`yyyy/mm/dd`, typed or picked from a calendar) and a
> time (`hh:mm:ss`) in local time or UTC and press
> **Fetch**: every stream of the group that matters for the range is read,
> one after another, and the events are listed in time order as they arrive
> ("time / stream / message"). Throttling and network errors are retried;
> streams that still fail are listed separately while the rest is shown.
> The list draws only the visible rows, so a million events scroll smoothly.
> The log filter narrows the fetched events to those whose message contains
> a text (ignoring case), on your machine, without calling AWS again.
> An optional disk cache (off by default, the gear button in the top bar)
> keeps fully successful fetches on your machine and shows a range it already
> holds without calling AWS. A row opens below itself to show its whole
> message; the reason why **Fetch** is unavailable is shown under the field
> it belongs to, a failed fetch is an error alert above the list that says
> what happened and what to do next; closing the window while fetching asks
> first.

Required IAM permissions: `logs:DescribeLogGroups`, `logs:DescribeLogStreams`
and `logs:GetLogEvents`.

## Install

To use the app on a Mac with Apple Silicon (for example a company PC), install
the released `.dmg` from
[GitHub Releases](https://github.com/kdnakt/local-sights/releases): see
[docs/install.md](docs/install.md) for installing, the first-launch warning
(the app is ad-hoc signed, not notarized by Apple), updating and going back to
an earlier version. The AWS
preparation in [Prerequisites](#prerequisites) is needed there too. The rest of
this README is for development.

## Layout

| Path                                               | What it is                                                                                                           |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `crates/local-sights-core/`                        | GUI-independent library: validation, time range, paging, gateway (AWS SDK behind a trait), fetch flow, session state |
| `crates/local-sights-core/examples/fetch_check.rs` | Developer-only command-line check against real AWS                                                                   |
| `src-tauri/`                                       | Tauri app (`local-sights`): commands and events only                                                                 |
| `src/`                                             | Screen (Vite + React + TypeScript)                                                                                   |
| `scripts/release/`                                 | Release version check (`npm run check:version`)                                                                      |
| `docs/`                                            | Installing the released app and making a release                                                                     |

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
  `aws sso login --profile <name>` first, or sign in from the app (see
  [Sign in with SSO](#sign-in-with-sso), which needs AWS CLI v2). The app
  never reads, stores or logs your credentials.

## Run the app

```bash
npm ci
npm run tauri dev      # development window with hot reload
npm run tauri build    # release bundle under target/release/bundle/
```

Or, for development, install the binary from source (builds the screen
first):

```bash
npm ci && npm run build
cargo install --path src-tauri --features custom-protocol
```

At launch the "Default" profile (the SDK's default credential chain) is
selected, and when the SDK has a default region (`AWS_REGION`,
`AWS_DEFAULT_REGION` or the `[default]` section of `~/.aws/config`) that
region is selected too and its log groups are listed at once; the previous
session's choice is not remembered. The start and the end begin as yesterday
`00:00:00` and today `23:59:59` in the local time zone (the calendar day of
the OS clock, so the range is whole days also across a daylight saving
change); change them as needed. Choose another profile and a region (the
profile's default region is filled in when it has one) from the dropdowns in
the top navigation, pick a log group in the left pane (its width can be
dragged, or changed with the arrow keys on the handle between the panes),
then set the start and the end: a date (`yyyy/mm/dd`, typed in the
field or picked from its calendar, which offers no day after today in the
time zone shown; a date typed by hand is not limited, and a future range
simply finds no events; a hyphen-separated date is not accepted)
and a time (`hh:mm:ss`, in the time zone chosen in the top navigation; the
end second is included up to its last millisecond). A time typed as `hh:mm`
is completed when the field loses focus (`:00` for the start, `:59` for the
end), and an empty time becomes `00:00:00` for the start and `23:59:59` for
the end; the field shows the completed value. Then press **Fetch** or Enter.
What keeps **Fetch** unavailable is shown in red under the field concerned
(the log group under the heading, the order of the range under the end
date). The log list shows the times as `yyyy/mm/dd hh:mm:ss.SSS`.
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
read, never credentials (and, from `~/.aws/config`, whether a profile has
`sso_session` / `sso_start_url` and the name in `source_profile`, for the
SSO sign-in below; never their values).

## Sign in with SSO

When the SSO session has expired, the log group pane or the error above the
log list says the credentials could not be used. If the selected profile is
an SSO profile, or takes its credentials from one through `source_profile`
(up to five steps, as with an AssumeRole profile), that message carries a
**Sign in with SSO** button:

1. Press it. The app starts `aws sso login` of the AWS CLI (no shell; the
   only arguments are `sso login` and, when needed, `--profile <name>`) and
   shows a dialog. With **Default** selected the CLI picks the profile
   itself (`AWS_PROFILE`, otherwise `default`), unless that profile takes
   its credentials from another one, which is then named with `--profile`.
2. Approve the sign-in in the browser the CLI opens. If no browser opens,
   click the URL in the dialog (only an `https://` address on
   `*.amazonaws.com` or `*.awsapps.com` is shown and opened) and enter the
   verification code when one is shown (newer CLIs print none).
3. On success the dialog closes, "Signed in. Reloading the log groups."
   appears above the log list and the log groups are listed again with the
   new token (no restart needed). A failed fetch is not repeated: press
   **Fetch** again.

**Cancel** stops the CLI; after five minutes without approval the CLI is
stopped and the dialog says the sign-in timed out. If the CLI fails, the
dialog shows its exit code (never its output) and how to sign in from a
terminal instead. While signing in, the profile, the region, the log group
list, **Reload**, **Fetch** and the settings are disabled. Closing the
window or quitting the app stops a running CLI; it does not ask first.

The AWS CLI is looked for at `/opt/homebrew/bin/aws` (Homebrew), then
`/usr/local/bin/aws` (the official installer); `PATH` is not used, because
an app started from the Finder or the Dock does not get the terminal's
`PATH`. If neither exists, the dialog says the AWS CLI was not found:
install [AWS CLI v2](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html),
or run `aws sso login` in a terminal and press **Reload**.

The app itself calls no SSO API and never reads or writes the SSO token
cache (`~/.aws/sso/cache`); the CLI signs in and the AWS SDK reads the token.
The CLI's output is read line by line only to find the URL and the code; it
is not kept, logged or written anywhere.

## Manual GUI check (walking-skeleton checkpoint)

Run these on your own machine with your own AWS credentials (`npm run tauri dev`):

- [ ] The app starts and shows the profile and region selectors, the log group pane, the selected log group ("Not selected"), the time inputs, the **Fetch** button and the reasons why Fetch is unavailable.
- [ ] With valid conditions, pressing Enter in any input starts the fetch (same as clicking **Fetch**).
- [ ] While fetching, every input and **Fetch** are disabled and the status line says it is fetching.
- [ ] The fetched rows appear with the time to the millisecond (`yyyy/mm/dd hh:mm:ss.SSS`) in the chosen time zone (Local at launch, U4).
- [ ] A long message, or one with line breaks, is shown on one line and cut with an ellipsis (…) at the edge of the column.
- [ ] The status line shows the event count, and for a range with no logs says explicitly that there are 0 events.
- [ ] On an error (for example a misspelled log group, or a profile without a default region), the error area above the list says what happened, what to do next and the safe detail, which contains no secret or access key ID; the status line says only what happened (U7); rows fetched before the error stay visible.
- [ ] After a fetch finishes, the inputs are enabled again and fetching again replaces the previous rows.

Connection and log groups (U2):

- [ ] At launch the profile is "Default" (the SDK's default credential chain; the top bar reads "Profile: Default"); with a default region in the SDK's settings the region is selected and the log groups are listed without any click, otherwise the region is unselected and the pane asks for a region. The profile list starts with "Default", followed by your profiles in name order. The previous launch's choice is not restored.
- [ ] Choosing a profile with a default region fills in the region and the log group list loads ("Loading…" while pages arrive; groups can be selected meanwhile).
- [ ] Choosing a profile without a default region leaves the region unselected and the pane asks for a region; choosing one loads the list.
- [ ] The filter narrows the list case-insensitively (e.g. `myfunc` finds `/aws/lambda/MyFunction`); a filter with no match says so; an account without log groups says there are none.
- [ ] Selecting a group (click, or arrow keys then Enter/Space) shows its name next to the time inputs, even when the filter hides it. **Reload** re-reads the list and keeps the selection and the shown logs.
- [ ] With logs shown, changing the profile or region asks "The logs shown will be cleared. Change the connection?" with focus on **Cancel**; **Cancel** or Escape keeps everything, **Change** clears the logs, count and error, deselects the log group and loads the new list.
- [ ] While fetching, the profile, region, log group list and **Reload** are disabled.
- [ ] With a profile that lacks permission (or an expired SSO login), the pane shows that the list is incomplete, what happened, what to do next and the safe detail (U7), and the app keeps running; you can choose another profile or region.
- [ ] If `~/.aws/credentials` (or config) exists but cannot be read, a notice near the profile selector names the file kind only (no path, no content).

Whole-group fetch (U3):

- [ ] There is no stream input; **Fetch** is available once a log group is selected and the times are valid.
- [ ] While the streams are listed, the status line says "Listing streams…" with the number selected so far; then "Fetching… n/m streams" with the events so far.
- [ ] For a group with several active streams, the rows of all streams are mixed in time order (equal times: by stream name, then the order within the stream) with the stream name in its own column.
- [ ] With close to a million events, scrolling (mouse, or moving the selected row with the arrow keys, Page Up, Page Down, Home, End since U7) stays responsive and reaches the first and last rows.
- [ ] While a fetch adds rows, a list scrolled down keeps the row at the top where it is; a list at the very top stays at the top.
- [ ] When some streams fail (for example with a role that may not read one stream), the status line shows "Failed in N streams"; pressing it (or Tab to it and Enter/Space) opens the list of stream names, each with what happened, what to do next and the safe detail (U7), focus on **Close**; **Close** or Escape closes it, and fetching again empties it.
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

- [ ] The gear button in the top navigation opens the settings dialog with focus on the checkbox; it shows the cache location (on macOS under `~/Library/Caches/dev.local-sights.app/log-cache`) and the warning that logs may contain confidential information, checked or not. Tab cycles inside the dialog, Space toggles the checkbox, Escape or **Cancel** closes it without saving and focus returns to the gear button. The gear button cannot be pressed while fetching or while the connection-change dialog is open.
- [ ] Enable the cache and **Save**; restart the app and open the dialog again: it is still enabled (the settings file is under `~/Library/Application Support/dev.local-sights.app/`, owner-only, holding only the setting). The cache folder and its files are owner-only (`ls -ld` shows `drwx------`, files `-rw-------`).
- [ ] Fetch a range that ended more than five minutes ago: the status line says "Saving to cache" before the fetch ends. Fetch the same profile, region, log group and range again: the rows appear with "Shown from cache (AWS was not called)" and no stream counts, and no AWS call is made (for example, CloudTrail or a proxy shows none). The log filter works on these rows too.
- [ ] Fetch a range whose end is within five minutes of now: fetching it again goes to AWS (the newest five minutes are never cached); a part of it older than five minutes is served from the cache.
- [ ] Widen a cached range: it goes to AWS, and afterwards the wider range is served from the cache.
- [ ] Damage a cache file (for example cut its last line with `truncate -s -10 <file>`): the next fetch of that range says "The cache could not be read, so the logs were fetched again" and the file is replaced by a good one.
- [ ] **Clear cache** removes the cache files at once ("The cache was cleared.") and **Cancel** does not bring them back; unchecking the box and **Save** also removes them and closes the dialog.
- [ ] With close to a million events, note how long "Saving to cache" lasts (the fetch ends only after the write; the design sets no bound, NFR2), and that the screen keeps scrolling meanwhile.

Screen polish (U7):

- [ ] Clicking a row's line opens its whole message right below it; clicking it again closes it; several rows can be open at once. The text keeps its line breaks, is not reformatted (JSON stays as received), wraps at the edge (also a long word without spaces), and a message such as `<b>bold</b>` shows the tags as text. More than 20 lines scroll inside the expansion. The text can be selected and copied with Cmd+C; clicking inside it does not close the row.
- [ ] In the list, ↑ / ↓ move the selected row (shown in the selection color), Page Up / Page Down by a screenful, Home / End to the first / last row (also with close to a million events); Enter or Space opens or closes the selected row. With the selected row's long expansion open, Tab enters it (↑ ↓ and Space then scroll it), Escape or Shift+Tab goes back to the list. A screen reader announces the list as a grid with the selected row.
- [ ] Opening, closing and changing the size of the window keep the row at the top in place; the rows above an opened row do not move. Expanded rows stay open when the log filter changes (a hidden row opens again when shown), when the time zone is switched and while a fetch adds rows; they all close when a new fetch starts or the connection changes.
- [ ] With close to a million events and some rows open, scrolling and opening rows stay responsive.
- [ ] Errors read as words in the error area above the list (a failed fetch), in the log group pane (a failed list, pointing to **Reload**) and in the failure list (a failed stream), each with "Details: …" when there is a safe detail; the status line shows only what happened. No error is shown by color alone. The next action names the button as shown: "[取得]" / "[再読み込み]" in Japanese, "press [Fetch]" / "press [Reload]" in English (lowercase inside a sentence, capitalized when it starts one).
- [ ] Closing the window, Cmd+Q and Quit in the app menu: while not fetching the app closes at once; while fetching (and while "Saving to cache") a dialog says "Fetching is in progress." with focus on **Keep fetching**. **Keep fetching** or Escape closes the dialog and the fetch goes on; **Close** stops the fetch and ends the app without asking again. When the fetch ends while the dialog is open, the dialog stays; **Close** then ends the app.
- [ ] Quit in the Dock menu: Dock の終了は確認できない（既知の制約）. It ends the app at once even while fetching (the fetched logs are lost and nothing is written to the cache); check that it does so without hanging.
- [ ] The app menu's Edit items (Undo, Redo, Cut, Copy, Paste, Select All) work with their shortcuts: Cmd+C copies the selected text of an expanded message, Cmd+V pastes into the inputs.
- [ ] Opening more than 10,000 rows at once is refused with "Up to 10,000 rows can be open at once. Close a row to open another." below the list (optional; tedious by hand).
- [ ] After **Close** during "Saving to cache", the next launch shows the previous cache unchanged (fetching the same range is served as before, or fetched again); a leftover `*.cache.tmp-*` file in the cache folder goes away with the next save or **Clear cache**.
- [ ] Switching the OS between light and dark mode recolors the open window at once (no restart); dialogs and the selected row stay readable.
- [ ] At the minimum window size, 1024×640, the top bar, the log group pane, the conditions, the list and the status line do not overlap and there is no horizontal scroll bar; the window cannot be made smaller.
- [ ] With the OS language set to Japanese every text is Japanese, otherwise English; Tab goes profile → region → time zone → gear button → the pane's "<" button → log group search → log group table → **Reload** → the pane handle → **Fetch** → log filter → start date → its calendar → start time → end date → its calendar → end time → list, with the focus always visible (with the pane closed: gear button → the ">" button → **Fetch**).
- [ ] Diagnostics appear only on standard error when the app is started from a terminal (`npm run tauri dev`); no log file is written anywhere.

Console-style screen (261010-console-ui-polish):

- [ ] Every visible control is a Cloudscape component: the black top navigation with the profile, region and time zone dropdowns and the gear button; the "Log groups" container with its search field, single-selection table and **Reload**; the right container headed by the selected log group (or "Log group: none"; the prompt "Choose a log group." appears once, on the line below the heading) with the primary **Fetch**, the "Filter messages" field (its label on the same line as the "Start" and "End" labels, so the three fields share one top edge) and the start / end date and time fields; the log list; the status line with its status indicators; the dialogs as modals; the alerts.
- [ ] The calendar button of a date field opens a calendar (`Tab` to it, Enter, arrow keys, Enter, Escape all work); a picked day lands in the field as `yyyy/mm/dd`, and typing `2024/03/01` works too. Type `2024-03-01`: the field normalizes it to `2024/03/01` (the hyphen is taken as a separator); note what happens and record it in the code summary (R-07).
- [ ] Type `10:00` in the start time and leave the field: it reads `10:00:00`; the same in the end time reads `10:00:59`. Leave a time empty and press Enter or **Fetch**: the start becomes `00:00:00`, the end `23:59:59`, and the fetch starts when the rest is valid.
- [ ] Without a log group the reason sits under the heading; a start after the end puts "End must be after start." under the end date; a daylight-saving gap time puts its reason under the time field. The old "Fetch is unavailable because:" list is gone.
- [ ] A failed fetch (for example, a profile without permission) shows an error alert above the list with three lines (what happened, what to do, details) and a close button; closing it keeps the status line's short sentence. The alert does not close with Escape.
- [ ] Switching the OS between light and dark mode recolors the whole window at once, the log list included; the top navigation stays dark.
- [ ] With the OS language set to Japanese, the calendar's month and weekday names and the modal close buttons are Japanese too.
- [ ] At 1024×640 the two panes and the condition row (filter left, start and end right) fit without overlap; if the date and time fields wrap below the filter, that is the intended fallback.
- [ ] The calendar of a date field opens with all seven weekday columns and both month buttons (previous, next) visible; nothing is cut at its right edge.
- [ ] Started from a terminal (`npm run tauri dev`) with the WebView inspector open, no Content-Security-Policy violation is reported at startup or when a calendar, a dropdown or a dialog opens (the CSP allows the embedded fonts of Cloudscape with `font-src 'self' data:`).
- [ ] The Dock, the window title and the window switcher (Cmd+Tab) say "Local Sights", and the Dock and the Finder show the new icon (a dark blue tile with log lines and a magnifying glass); the first launch after the change may need the Dock to refresh its icon cache.
- [ ] The log group pane starts at about a quarter of the window; the handle between the panes can be dragged narrower (down to 20% of the window, where long names are cut with "…" and show in full on hover) or wider (up to 50%), and with the focus on it the Left and Right arrow keys move it by 1%; the width is the same after a restart.
- [ ] The "<" button in the pane's heading closes the pane: only a slim strip with a ">" button remains at the left edge, the log list takes the rest of the width, and the focus is on that button. The ">" button opens the pane again at its previous width, with the focus on the "<" button. A pane closed when the app is quit is still closed at the next launch.
- [ ] At launch the start date is yesterday with the time `00:00:00` and the end date is today with the time `23:59:59`, in the local time zone (the days follow the local calendar day of the OS clock; try it around midnight too); switching to UTC rewrites both, and choosing a log group is all **Fetch** still needs.
- [ ] In the Dock the icon is the same size as the other apps' icons: its rounded tile fills the central 824 of the 1024 px with a transparent margin of 100 px on every side, like Apple's app icon grid (the Dock may need a restart to drop its cached icon).
- [ ] In the calendar of the start date and of the end date, tomorrow and every later day are dimmed and cannot be picked (clicking them does nothing), while today and earlier days can; with **UTC** selected, "today" is the UTC date (around local midnight it differs from the local date by a day). A future date typed by hand is still accepted.
- [ ] With 1,000 or more rows held (or a fetch running and adding rows), the status line ("Fetching… n/m streams…", the event count, "Filtered: N of M events") stays visible at the bottom edge of the right container; only the log list scrolls, inside the space left between the condition row and the status line. Check it at the minimum window size, 1024×640, with a failed fetch's error alert shown above the list, and while making the window taller and shorter.

Sign in with SSO (261011-sso-login), on the company Mac with AWS CLI v2:

- [ ] With an expired SSO session and a profile that has SSO settings directly (`sso_session` or `sso_start_url`) selected, the log group pane's error shows **Sign in with SSO**; pressing it opens the dialog, the browser asks for approval, and after approving the dialog closes, "Signed in. Reloading the log groups." appears and the list loads without restarting the app. **Fetch** then works with the new token.
- [ ] The same with an AssumeRole profile (`source_profile` pointing to an SSO profile): the sign-in is for the SSO profile, and afterwards the AssumeRole profile's list loads.
- [ ] With a fetch that failed for the expired session, the error above the list also shows **Sign in with SSO**; after signing in the fetch is not repeated until **Fetch** is pressed.
- [ ] A profile with access keys, or an error other than expired credentials (for example a network error), shows no **Sign in with SSO**.
- [ ] When the browser does not open by itself (for example, close the browser window the CLI opened, or set `BROWSER` to a command that does nothing before `npm run tauri dev`), the dialog's URL opens the sign-in page, and the verification code shown in the dialog (when there is one) completes the sign-in.
- [ ] **Cancel** while waiting closes the dialog and leaves no `aws` process: check in Activity Monitor (or `pgrep -fl "aws sso login"`) that none is left.
- [ ] Waiting five minutes without approving shows "The sign-in timed out. Try again." and leaves no `aws` process.
- [ ] Cmd+Q, Quit in the app menu, closing the window and Quit in the Dock menu while the dialog waits end the app without asking, and leave no `aws` process.
- [ ] With the AWS CLI moved away from both `/opt/homebrew/bin/aws` and `/usr/local/bin/aws` (or on a Mac without it), the dialog says "AWS CLI was not found. Install AWS CLI v2." with the terminal alternative.
- [ ] While the dialog waits, the profile, region, log group list, **Reload**, **Fetch** and the gear button cannot be used, Escape and the dialog's close button do not close it, and **Cancel** is reachable with Tab; after the dialog closes, the focus is back on the **Sign in with SSO** button (or on **Reload** when that button is gone).
- [ ] Started from a terminal (`npm run tauri dev`), standard error shows only "sso login N ended: …" lines for the sign-in: no URL, code, token or CLI output.

## Performance check of the log list

The log list is still the app's own virtual-scroll list, styled with
Cloudscape's design tokens (NFR2: Cloudscape's Table draws every row). A
Cloudscape Table prototype exists for measuring, and a developer-only
harness feeds generated rows through the screen's `getRows` boundary without
the core. Run on your own machine:

```bash
# one million generated rows in the current list (virtual scroll)
VITE_LOCAL_SIGHTS_PERF_ROWS=1000000 npm run tauri dev
# the same rows in the Cloudscape Table prototype
VITE_LOCAL_SIGHTS_PERF_ROWS=1000000 VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape npm run tauri dev
# rows arriving 100,000 at a time every two seconds, like a running fetch
VITE_LOCAL_SIGHTS_PERF_ROWS=1000000 VITE_LOCAL_SIGHTS_PERF_GROW=1 npm run tauri dev
```

With the harness on, the list shows the generated rows (messages of 80 to
400 characters, plain text and JSON) and the "Filter messages" field filters
them on the screen side; the status line keeps showing the real (empty)
session. The harness exists only in development builds: a production build
(`vite build`, `npm run tauri build`) ignores these variables. Measure, with a stopwatch where needed: typing a filter until the
result appears (target: within 100 s), one wheel notch, Page Down, End and
opening a row until the screen changes (target: within 1 s), typing into the
filter field while rows arrive (target: within 1 s), and that the app does
not run out of memory. Record the numbers in
`aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/code-summary.md`
("性能測定"). The prototype replaces the list only if it meets every target.

## Check against real AWS from the terminal

`fetch_check` uses the same validation and fetch flow as the app and reads
the whole log group. Its `--start` and `--end` are always UTC and it prints
UTC times; the time zone switch belongs to the screen only (U4:BR3.5), and
so does the log filter (U5:BR3.5). Run it only on your own machine with your own
credentials; tests and CI never run it.

```bash
cargo run -p local-sights-core --example fetch_check -- \
  --profile dev --region ap-northeast-1 --log-group /aws/lambda/my-function \
  --start '2024/01/02 03:00:00' --end '2024/01/02 03:59:59'
```

`--region` is required. It is the region the fetch connects to and also sets
the ceiling on `GetLogEvents` calls, half the region's AWS limit (12 calls a
second in Tokyo; 5 when the region is unknown), so leaving it to the profile
would understate the speed.

Events go to standard output in time order
(`UTC time<TAB>stream name<TAB>message`). Standard error gets the one-line
fetch summary, for example
`local-sights: fetch summary streams=180 calls=212 throttled=3 elapsed_ms=18450 max_in_flight=8 status=Completed`
(numbers and the status only; the app writes the same line when started from
a terminal), then the event, stream and failed-stream counts, then each
failed stream (or an incomplete stream listing) with its failure kind and
safe detail. Exit code 0 means success, 1 that something failed, 2 invalid
arguments.

## App icon

The icon is drawn in `src-tauri/icons/source/icon.svg` (a rounded dark blue
tile with log lines and a magnifying glass). Its dimensions follow Apple's
app icon grid: on the 1024 × 1024 canvas the rounded tile is 824 × 824,
centred, with a transparent margin of 100 px on every side and a corner
radius of 185 px; the drawing inside the tile is the original 1024 drawing
scaled by 824 / 1024. A tile that filled the whole canvas looked larger than
the other apps' icons in the Dock. The files Tauri bundles are generated from
the SVG; do not edit them by hand:

```bash
npm run icon:render   # icon.svg -> src-tauri/icons/source/icon.png (1024x1024, transparent corners and margin)
npm run tauri icon src-tauri/icons/source/icon.png -o /tmp/local-sights-icons
cp /tmp/local-sights-icons/{32x32.png,128x128.png,128x128@2x.png,icon.icns,icon.ico,icon.png} src-tauri/icons/
```

`icon:render` draws the SVG with `@resvg/resvg-js` (a dev dependency) and
refuses to write the PNG unless the four corner pixels and the middle of each
100 px margin band are fully transparent and the centre is opaque; macOS's
`qlmanage` is not used because it fills the transparent corners with white.
Only the six files `tauri.conf.json` refers to are copied back; the Android,
iOS and Windows Store images `tauri icon` also writes are not kept.

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
npm run test:coverage           # screen tests with line coverage (fails below 80%)
cargo llvm-cov -p local-sights-core --fail-under-lines 80  # library line coverage (needs cargo-llvm-cov)
cargo fmt --check
cargo clippy --workspace
npm run typecheck && npm run lint && npm run format:check
npm audit
npm run check:version           # same version in Cargo.toml, Cargo.lock, package.json, tauri.conf.json
cargo test -p local-sights-core --example fetch_check  # fetch_check argument parsing (--region is required)
```

Against real AWS, by hand only: `fetch_check` needs `--region` (see "Check
against real AWS from the terminal") and prints the one-line fetch summary
(`streams=… calls=… throttled=… elapsed_ms=… max_in_flight=… status=…`) to
standard error. To compare the speed with `main`, run both with the same
profile, log group, time range and `AWS_REGION`, giving this version the same
region in `--region` too, three times each under `time`, and compare the
median real time and the event count.

## Releasing

A tag `vX.Y.Z` pushed on a commit of `main` builds the Apple Silicon `.dmg`
and publishes it on GitHub Releases. See [docs/releasing.md](docs/releasing.md)
for raising the version, tagging, and what happens when a run fails or is run
again.

## Secrets

Never commit AWS credentials, access key IDs, session tokens or SSO tokens.
Errors and diagnostics show only safe details (error kind, API name, request
ID, profile name, log group, log stream); anything that looks like an access
key ID, secret access key or session token is masked as `[REDACTED]`.

## License

Apache License 2.0. See [LICENSE](LICENSE).
