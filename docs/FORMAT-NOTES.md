# .dvw / .vsm format notes

Everything learned about the formats, each entry with the corpus file(s) that show it.
When Courtside and openvolley disagree, check the file by hand, decide who is right, and record it here.

Corpus files are referred to by their manifest id (`C001`…; the id → file mapping is only in the private `testdata/manifest.toml`). The seed files: C001–C007 and C008–C017 are DataVolley 4 `.dvw` files from two tournaments, C018 is a VolleyStation `.dvw` export, and C019–C036 are VolleyStation `.vsm` files.

## Sections

- All 18 seed files have the same 14 sections in the same order: `3DATAVOLLEYSCOUT`, `3MATCH`, `3TEAMS`, `3MORE`, `3COMMENTS`, `3SET`, `3PLAYERS-H`, `3PLAYERS-V`, `3ATTACKCOMBINATION`, `3SETTERCALL`, `3WINNINGSYMBOLS`, `3RESERVE`, `3VIDEO`, `3SCOUT`. No lines before the first header. (all)
- Empty sections are just the header line followed by the next header (`3ATTACKCOMBINATION`, `3SETTERCALL`, `3RESERVE`, `3VIDEO` in C001–C007, C008–C017).
- `[3SET]` always has 5 rows; unplayed sets are `True;;;;;;`. Partial scores may be space-padded: `8 -7` (C001), ` 7- 8` (C018). DataVolley pads only the home score to two characters (`{home:<2}-{visiting}`: `8 -7`, `16-7`, `7 -10`); VolleyStation right-aligns both (` 7- 8`, `12-16`). Partial scores are the score after the point on which either team first reaches 8, 16 and 21, or 5, 10 and 12 in a deciding set (`3 -5;7 -10;7 -12;11-15`, C003 set 5). Unplayed sets may carry a duration of `0` (`True;;;;;0;`, C002). vb-engine regenerates the scores and `[3TEAMS]` sets won from the fold and reproduces them byte-exact on all 18 seed .dvw files (`dvw_round_trips_through_the_engine`).
- `[3SET]` duration (field 5): DataVolley writes ⌈(clock of the set's last point code − clock of its first serve − 1 s) / 60⌉ minutes: all 58 DataVolley sets in the seed corpus, including 20:01 → 20 (C004 set 2) and 18:01 → 18 (C013 set 1). VolleyStation's durations (C018: 24, 23, 28) don't follow its clock, which jumps back mid-set, so vb-engine keeps them as written. The 2026-10-19 batch shows DataVolley doesn't always follow the rule either: x:01 spans rounded up (C052, C053, C055), 31 minutes for a 77-minute clock span and 23 for 49 (C060, C061, which DataVolley didn't time from the scout clock), and empty durations (C037 set 5, C040, C059 set 4). So export keeps a set's duration as the file wrote it, empty or not, unless that set's scores change, and keeps `[3TEAMS]` sets won as written (empty when no set was played: C049, C062; `null` in C075; counting the golden set in C043) unless some set's scores change (`unchanged_sets_keep_what_the_file_says`).
- Player-table starting zones (`[3PLAYERS-*]` fields 3–7): a player brought on by a `c` code gets `*` for that set, and so does a libero who touches the ball in it (no code brings a libero on; every libero `*` in the seed corpus matches a touch). vb-engine writes both. Exceptions: four rows where the sheet has `*` and no code brings the player on (the missing substitution codes in C011, C014, C016; see "Validator"), kept from the sheet, and C009, whose home libero touches the ball in set 1 though the sheet says they didn't play it (the file's set 1 sheet is wrong, see "Engine fold"); the engine writes `*` there.
- `[3VIDEO]` line is `Camera0=<absolute local path>` (C018). That path is personal data; don't echo it into exports by default. Courtside keeps the first camera's path as the match's video (season schema v6, `match.video_path`) and writes `[3VIDEO]` empty on export unless the scout includes it, then as `Camera0=<path>` (the file's own line byte for byte when the path is unchanged; `the_video_path_is_exported_only_when_asked`).

## Header ([3DATAVOLLEYSCOUT])

- `FILEFORMAT: 2.0` in all files.
- `GENERATOR-PRG: Data Volley` / `GENERATOR-REL: Release 4.2023.03` (C008–C017) and `Release 4.2025.01` (C001–C007), `GENERATOR-VER: Media`.
- VolleyStation writes `GENERATOR-PRG: VolleyStation`, `GENERATOR-VER: Professional`, empty `REL` and `NAM`, keeping the trailing space after the colon (`GENERATOR-REL: `). Must round-trip that space. (C018)
- Date order seen: `MM/DD/YYYY` in the DataVolley files (C001–C007, C008–C017), `DD/MM/YYYY` in the VolleyStation one (C018). Same in `[3MATCH]` field 1. Ambiguous for day ≤ 12. The order does **not** follow from `GENERATOR-PRG`: DataVolley writes the scouting PC's Windows locale and VolleyStation a per-season setting (DD/MM, MM/DD or YYYY/MM/DD). So the corpus orders are those PCs' settings, not rules. Dates in one file can even differ in order: C006 and C007 have `GENERATOR-DAY` day-first (`01/08/2026 12:04:59`, `:` in the time) but `LASTCHANGE-DAY` and the match date month-first (`08/01/2026 13.43.58`), so the file was created on one PC and saved on another. `Dvw::match_date` therefore never borrows another date's order: serial first, then the written date if only one order is valid (`Date::parse_written`, strict: `A/B/YYYY` or `YYYY/MM/DD`, one separator), then the one reading within 14 days of an unambiguous `GENERATOR-DAY`/`LASTCHANGE-DAY`; otherwise none, and `courtside validate` warns that the order is ambiguous. Every corpus file resolves (17 by serial, C018 unambiguous).
- Both programs write the full `GENERATOR-*` and `LASTCHANGE-*` blocks (`DAY`, `IDP: DVW`, `PRG`, `REL`, `VER`, `NAM`; C001–C007, C018). A match coded in the console writes both too, `PRG: Courtside`, stamped with the match's creation time in UTC as `MM/DD/YYYY HH.MM.SS` (`courtside_store::template`). Whether DataVolley 4 needs them is unknown until the exit test.
- `[3MATCH]` line 2, field 3 (DataVolley only) is a spreadsheet date serial, day 0 = 1899-12-30: `46234` = 2026-07-31. It matches the written date (read month-first) in all 17 DataVolley files, so it is the most reliable date source. `12345` is a known placeholder (per openvolley). Empty in VolleyStation exports (C018). `Dvw::match_date` uses serial → generator order → unambiguous parse, in that order.

## Encodings

- `[3MATCH]` field 9 is the Windows code page: `1252` in the seed files. The 2026-10-19 batch adds files that don't say or say wrong: code page `1` (Windows' "PC default", DataVolley 3.x) on a Shift_JIS file (C067) and a Windows-1251 one (C086); an empty code page on VolleyMetrics Portal files, which are UTF-8 (C074); C065 says 1252 but is UTF-8. With no code page or `1`, Courtside guesses (`TextEncoding::guess`): valid UTF-8 → UTF-8; many high bytes in 0x81–0x9F that decode cleanly → Shift_JIS; most high bytes in letter runs of 3+ (whole words, 0xC0–0xFF) → Windows-1251; a byte inside a word that is a letter in 1250 but a symbol or unassigned in 1252 (0x8F `Ź`, 0xB3 `ł`, 0xBF `ż`, …), or a word ending in 0xC6/0xE6 (`Ć`/`ć`; `Æ`/`æ` in 1252 hardly ends a word) → Windows-1250 (the 2026-10-27 batch: three VolleyStation files with code page `1` and Croatian `-IĆ` surnames, one with 0x8F, C127, C137, C139); else 1252. Danish and Norwegian `æ ø å` inside words stay 1252. A declared code page is still trusted. C071 is double-encoded (1250 text read as 1252, saved as UTF-8): no reader gets its names right.
- 13 files are pure ASCII. 5 contain raw cp1252 bytes in player names, which are invalid UTF-8: `í` = 0xED (C002), `ò` = 0xF2 (C009, C012, C015, C017). Tools that sniff UTF-8 (grep, `file`) treat these as binary.
- DataVolley 4 stores a Unicode copy of text fields: `\x0F` (invisible control byte), a digit `n` (`2` or `4`), then `n` hex digits per character, each a **Unicode code point** (not a cp1252 byte; they coincide below U+0100). E.g. first name `Siobhán` → `\x0F253696F6268E16E`; empty → `\x0F2`. All 1,536 such fields in the seed corpus use width 2 and decode to exactly the plain column (verified in `typed_headers_are_sane`). Columns: `[3MATCH]` 12–13 (league, phase; openvolley also reads 14), `[3TEAMS]` 6–8 (name, coach, assistant), `[3MORE]` 6–9 (referees, city, arena, scout), `[3PLAYERS-*]` 17–19 (last, first, nickname). 0-based. VolleyStation omits them (C018).
- When a letter isn't in the file's code page, DataVolley writes its base letter in the plain column and the true letter only in the Unicode copy (plain `C` / copy `Č`, plain `e` / copy `ě`: C052 8 rows, C045). The "differs from its Unicode copy" check accepts that, and `?` (`same_name` in `courtside-ffi`).
- Reading prefers the Unicode copy when it decodes to non-empty text, else the plain column (same as openvolley). Writing must keep both in step.
- Tokeniser works on bytes, so encoding is irrelevant to round-trip; decoding is task 4.

## Line endings

- DataVolley: CRLF throughout, trailing CRLF at EOF (C001–C007, C008–C017).
- VolleyStation: LF throughout, **no** trailing newline at EOF (C018).
- Tokeniser stores each line's terminator, so both round-trip byte-exact.

## Field content

- Literal tab characters inside fields: `Surname Forename<TAB>` (coach) in `[3TEAMS]`, `Forename<TAB>` in `[3PLAYERS-H]` (C018). Never trim fields.
- Competition names can contain `|` (C008–C017: `CEV U20 ... 2024 | Men - Pool in IRL`).
- `[3PLAYERS-*]` columns (0-based): 0 team (0/1), 1 shirt number, 2 index, 3–7 starting zone in sets 1–5 (`1`–`6`, `*` = came on as sub, empty = did not play), 8 player id, 9 last name, 10 first name, 11 nickname, 12 special role (`L`, `C`), 13 role, 14 foreign (`True`/`False`/empty), 15 player id again (DataVolley), 16 ?, 17–19 Unicode copies. VolleyStation rows stop after column 15 (C018).
- Roles: 1 libero, 2 outside, 3 opposite, 4 middle, 5 setter, 6 unknown (openvolley's name; one row in C002). Also seen: empty (C007, C017) and the literal string `NaN` from VolleyStation for players with no role (C018, 4 rows; a JavaScript NaN leaking into the export). Treat empty and `NaN` as "no role"; round-trip them as written.
- Placeholder player rows exist: id `unknown-4`, last name `Unknown Player 4`, and the same **plain** text (no `\x0F`) in column 17 (C017). Not a DataVolley pattern; probably added by other software. The embedded-text decoder ignores fields without the `\x0F` prefix, so it falls back to the plain name.
- `[3MATCH]` line 1 (0-based): 0 date, 1 time, 2 season, 3 league, 4 phase, 5 home/away, 6 day number, 7 match number, 8 code page, 9 regulation (0 indoor sideout, 1 indoor rally point, 2 beach), 10 `Z`/`C` (zones or cones), 11 `0` in DataVolley, 12–14 Unicode copies.
- `[3MATCH]` line 2 (DataVolley): `2588;134;46234;;;;L;R;VIR;52515;` = federation match id, competition id, date serial, ?, ?, ?, `L`/`R` and `R`/`L` (court sides?), federation code (`VIR`, `CEV`), federation id. Only the serial is interpreted.
- `[3TEAMS]` (0-based): 0 id, 1 name, 2 sets won, 3 coach, 4 assistant, 5 shirt colour (Windows COLORREF int, `0x00bbggrr`: `255` red, `32768` green, `16777215` white), 6–8 Unicode copies. Corpus (2026-10-27): DataVolley 4 fills the colour in 41 of 46 files (`16016139`, a blue, is its usual one), DataVolley 3 too; VolleyStation leaves it empty in 55 of 59. Coach and assistant are "Last First" (VolleyStation reads the first word as the last name). Courtside (issue #49, schema 10) keeps a team's shirt colour as `#rrggbb`, writes it here and reads it on import (`colour_to_dvw`/`colour_from_dvw`), and writes `[3MORE]` city and arena from the match's venue for matches coded here. Sets won equals the number of scored `[3SET]` rows in every seed file.

## [3SCOUT] lines

- Columns (0-based), per openvolley's `read_main`; full table in `crates/dvw-codec/src/scout.rs`: 0 code, 1 point phase (`p`/`s`), 2 attack phase (`r`/`s`/`p`), 3 action (rally) id, 4–6 start/mid/end coordinate (`-1-1` = none), 7 time `HH.MM.SS`, 8 set, 9–10 home/visiting setter position, 11 video file number, 12 video seconds, 13 unused here (written empty), 14–19 home players p1–p6, 20–25 visiting p1–p6, then a trailing `;`.
- Columns 3 and 13 are empty on every line of all 18 seed files; write them empty.
- Column 11 is empty in every DataVolley file and `1` on all 822 lines of C018 (VolleyStation always writes `1`).
- Parity: all of these except 3, 8–10 and 13 match openvolley (datavolley 1.18.5, `c231092`) exactly on every line of all 18 seed .dvw files (`tests/parity.rs`). openvolley doesn't expose 8–10 raw (it derives set number and setter positions from the codes), and ignores 3 and 13.
- DataVolley Media files have no video column values (no video). VolleyStation fills a video-seconds column (`236`, `244`, … in C018).
- Courtside writes column 12 as whole seconds (truncated), like VolleyStation, on any line whose event has a video time the source line doesn't already hold (`export.rs`, `video_times_set_later_are_written`); column 11 stays as the source has it, empty on new lines. DataVolley's column 12 may be milliseconds, but no DataVolley file with video is in the corpus: a DV4 file coded on video settles the unit.
- Coordinates (columns 4–6), checked 2026-10-19 on the 26 corpus files that have them (DataVolley 3 and 4, VolleyStation, Volleymetrics, Click&Scout): a 100 × 101 grid, cell `x + (y − 1)·100`, written as four zero-padded digits; a missing mid or end point is `-1-1`, and a line with no marks has all three columns empty. openvolley's `dv_index2xy` gives each cell's centre in court units (court x 0.5–3.5, y 0.5–6.5, net at y 3.5; the grid reaches x 0.13–3.84, y −0.2–7.2 for balls out and serves from behind the line); `vb_engine::court` matches it on every cell. The frame is the play's own: every serve, attack, set and block starts in the bottom half (y < 3.5) whichever team plays it and in every set (100% of ~13,000 marked plays), and a team's zone 4 is on the left (attack start zone 4 at mean x 0.73, zone 2 at 3.24; serve end zone 1 at x 1.1, zone 5 at 2.9). A reception carries its serve's marks (reception start at the server, y ≈ 0.2) and a dig its attack's (dig start zone 4 at x 1.0, y 3.2), so they're in the server's and attacker's frame. Courtside stores the cells per event (season schema 7), copies a serve's marks to its reception and an attack's to its dig as it sets them (`Session::set_points`), keeps the source line's text when the marks are unchanged and writes them as above when they changed (`export.rs`, test `plays_are_marked_on_the_court`). A click with no drag starts the play in the middle of its start zone; the serve positions behind the baseline (zone 5 x 0.95, 7 x 1.25, 6 x 2.0, 9 x 2.7, 1 x 3.05, y 0.2) are the corpus means.
- Five-digit coordinates (2026-10-27 batch, 15 values in 9 VolleyStation files, e.g. C135 line 664 end `91103`): all two digits then `100`–`105`, which reads as x and y side by side for a point past the far end of the grid (inferred; the grid formula would put them nowhere). Courtside treats them as no mark (the raw line round-trips); openvolley keeps the text.
- Set end line: `**3set;...` with on-court columns empty and column 8 already the **next** set (all 18 files: `**1set` shows `2`, `**5set` shows `6` in C003, C004, C015).
- `$$&H#` / `$$&H=` (team prefix `*` or `a`) are green codes: a point credited to (`#`) or charged against (`=`) a team without a player touch, e.g. `*17AH#` then `a$$&H=` then `*p05:03` (C016). Every seed file has them: 87–181 per DataVolley file, 107 in C018, where VolleyStation also writes them for a rally with a winner but no rally-ending play and for each unit of an exported score correction. They are never player touches: the validator and stats skip them.
- When the green code is written (corpus, 2026-10-24, #23; DataVolley 4 and VolleyStation agree): it pairs a rally's last touch that has no counterpart from the other team. The winner's `S#` or `A#` with no touch after it gets the loser's `$$&H=` (DataVolley 4: 1,568 of 1,582 kills; VolleyStation 192 of 195); the loser's `S=`, `A=`, `E=` or `F=` gets the winner's `$$&H#` (1,006 of the serve errors, 902 attack errors, 201 set errors in DataVolley 4). A rally ending on a dig, reception or block error (`D=`, `R=`, `B=`, `B/`) gets none: that touch already pairs with the winner's. The green code takes the last touch's time and comes straight before the point code. A rally with only a point code gets nothing in DataVolley 4 and both greens in VolleyStation (rare); Courtside writes none.

## .vsm (VolleyStation native)

Seed: 18 files in `vsm/`, all VolleyStation, 2023-01 to 2024-03.

### Container

- Compact UTF-8 JSON on one line: no BOM, no whitespace, no trailing newline. Non-ASCII is raw UTF-8 (`è`, `ü`, `’`), control chars escaped (`\t`). (all)
- Parsing and re-serialising compactly **with key order preserved** reproduces every file byte-exact. So the lossless layer is `serde_json::Value` with `preserve_order` (`dvw_codec::RawVsm`). Key order is not alphabetical and varies between objects of the same kind (e.g. plays put `hitType` before or after `team`), so never re-serialise through a typed struct for round-trip.
- Top level: `gameType` (`"indoor"`), `version` (`1` in all), `startDate` (ISO 8601 UTC), `scout`, `team`, `attackCombinations`, `setterCalls`; optional `createdAt` (9 files), `comment` (5 files; free-text coach notes, may contain `\n`).
- `scout` = `{sets, video?}`. `video` = `{path}`, a local absolute path; missing in 1 file.

### Teams

- `team.home|away`: `name`, `shortCode`, `code`, `libero` (array of shirt numbers, 1–2 seen), `players`, `staff` (`coach`, `assistantCoach`, `assistantCoach2`, `therapist`, `medicalDoctor`, each `{firstName, lastName}`).
- Player: `_id`, `firstName`, `lastName`, `shirtNumber` (int), `code`; optional `position` (int), `nickname`, `isForeign`.
- Trailing tabs inside names here too (`"firstName":"…\t"`). Same source data as the tab-in-field quirk in C018's .dvw export.

### Sets and events

- `scout.sets[]`: `_id`, `startingLineup`, `events`, `score` (`{home, away}` final).
- Lineup: `{home|away: {positions: {"1".."6": shirtNumber}, setterAt: 1..6}}`. Position keys are strings, values ints.
- Each event has `_id`, `lineup` (state after the event), `score` (after), and exactly one of:
  - `exchange` = one rally: `{plays: [...], point: "*"|"a"}` (2,677 events)
  - `substitution` = `{team, playerIn, playerOut}` (ints) (254)
  - `timeout` = `{team}` (164)
  - none of these (6 events, only lineup + score): corrections. One changes a lineup (C019 set 3), four change the score by one point (C021 set 2 twice and set 3, C028 set 4), one changes nothing (C021 set 3).
- `_id`s are 24-hex MongoDB ObjectIds (timestamp in the first 4 bytes).

### Plays (`exchange.plays[]`)

- Always: `_id`, `team` (`*`/`a`), `player` (**string**, zero-padded `"03"`), `skill`, `hitType`, `effect`.
- Usually: `time` (all but 2), `originalTime` (most), `code` (95%).
- Optional: `skillType`, `startZone`, `endZone` (ints), `startSubZone`, `endSubZone` (`A`–`D`), `combination`, `targetAttacker`, `players` (int; blockers etc.), `special`, `custom`, `travelPath` (`[{x, y}, ...]` court coordinates).
- Naming vs DataVolley: `hitType` = DV skill type (position 5), `effect` = evaluation, `skillType` = DV skill subtype, `players` = num_players.
- **`code` is the keystrokes the scout typed, not the expanded DV code**: `7S.`, `.15A!` (dot/compound entry), as well as full forms like `a01RH-`. Missing on some plays. It cannot be used as the DV code; build the DV code from the fields.
- `time` vs `originalTime`: different scales (`4551` vs `6932022` on one play). Units unknown; probably video offset vs wall clock. Check against a video before relying on either.
- Values seen. skill: A S R D B E F. hitType: H M T Q O N U. effect: `- # + = / !`. skillType: mostly H, T, P, then rare `3 A B E C 2 R`. special: I Z S P A C U.

### Re-saved pairs

- C022/C023, C024/C025, C026/C027 are the same match saved twice. The later file has real post-match edits: changed `effect`/`skill`/`player`, plays added and removed, `_id`s regenerated on some plays. Good test data for the Correction model and for import diffing.

## Engine fold (vb-engine)

What DataVolley and VolleyStation write in the derived `[3SCOUT]` columns (8 set, 9–10 setter positions, 14–25 players on court), reproduced by `vb_engine::fold` on every corpus line except the two glitches below (`crates/vb-engine/tests/corpus.rs`).

- Columns show the state **after** the line. The `**1set` line already shows set 2, with all on-court columns empty and the setter positions carried over.
- A set's lineup comes from the `[3PLAYERS-*]` starting zones for that set, from the first line after the set end (the `>LUp` lines already show it). Setter positions on `>LUp` lines before the `z` code are leftovers: the previous set's (C001 set 2), or something else (`6`/`5` on C001 line 0, `0`/`0` on C002 line 0). The engine treats them as unknown.
- `z` without `>LUp` rotates the court so the current setter (last `P` code) is in that zone. It is usually one step after a side-out (1,307 of 1,407), but scouts also re-enter `z` to fix a rotation: 0, back one or back two steps (C001–C007, C008–C017). So the `z` code is authoritative and the rotation follows the setter, not a fixed step; the validator checks it against the side-out rule (see "Validator").
- `c` swaps a player in place. Media files never move the libero in these columns.
- DataVolley writes a setter change after a rotation (`*z4`, `*P8`, note no zero padding) meaning the rotation refers to the old setter (C007, C008, C011, C013, C014). VolleyStation writes `*z2`, `*P21` meaning the rotation refers to the **new** setter (C018 line 358). `vb_engine::Conventions::setter_change_follows_rotation`, set on import from `GENERATOR-PRG`.
- VolleyStation writes a rotation line with the substitutions straight after it already applied (C018: `*z4` then `ac07:03`; the `z` line already shows 3 for 7).
- Export (`Match::to_dvw`) writes these columns from the fold in DataVolley's convention, so the VolleyStation rotation lines (7 in C018) and both glitches below come out corrected; every other line is written back unchanged.
- Glitch: setter position `0` for ten lines of C007 set 3, with no code changing it.
- Glitch: C009 home on-court columns for set 1 (274 lines) contradict the player table's set 1 starting zones (three zones empty, a player with no starting zone on court). The table was probably edited after scouting. The engine follows the table.

## Validator (vb-engine)

What the six rules (`vb_engine::validate`) report on the corpus, and why (`validator_runs_on_the_corpus`; set `COURTSIDE_VALIDATOR_DETAIL=<text>` to list the lines).

- **Liberos are never in the six.** Both products keep the libero out of the on-court columns and exempt the team's liberos from "player not on court". Liberos are `L` in the special-role column; DataVolley also accepts role `1`, VolleyStation only `L`. In the corpus every role-1 player is also `L` (43 rows), so the two readings agree.
- **Libero rule:** a libero attack or block is an error (both products), a libero serve a warning (an error in DataVolley unless an option allows it; VolleyStation allows it). Seen: libero attacks in C002, C004 (home 2, line 295, not on court), C017; a libero block in C001 line 111.
- **Libero on court = team-sheet mislabel.** C007 home 10 is marked `L` but has starting zones 5 and 6 in sets 1–2 and serves and attacks from the six. Reported once per set as a warning, not on every touch.
- **"Player N is not on court"** (52 left after the exemptions; none are liberos) are scouting slips, not engine errors: a wrong shirt number (C003 line 633: `*12SH#` with 15 in zone 1, the next serve is `*15SH=`) or a missing substitution code (C016 set 3: the team sheet has 5 coming on, no `c` brings them on; set 2 line 507 `*c15:08` puts 8 on court twice). The message says which by checking the team sheet's starting zones for that set. Players missing from the team sheet get only the Code error.
- **Setter-position changes away from a side-out step are corrections, not mistakes.** DataVolley renumbers `z` codes after a setter change and scouts re-enter `z` by hand; VolleyStation writes a `z` for every setter-zone change. Typical corpus case, a double switch: `*c13:08`, `*c21:04`, `*P4`, `*z1` (C002 line 282; also C003, C006). Only the `z` codes straight after the point code count as the side-out step: if they contradict the side-out rule it is a warning (10 in the corpus: the wrong team rotated, e.g. C004 lines 882 and 887, or the knock-on of a missing point code, C001 line 79); any other change is `Info` (36). "Won a side-out but didn't rotate" stays an error.
- **Score corrections (2026-10-27 batch).** VolleyStation exports a corrected score as `$$&` codes and a point code per point changed, so the score can go down (`*$$&H= a$$&H# ap06:17` after 6:18) or jump. A run of rallies made only of `$$&` codes and point codes, one of which doesn't add exactly one point, is a correction: `Info` "score corrected from a:b to c:d", and the serving order isn't checked across it (a correction can change who serves). One such rally adding one point is a point without a play and is checked as usual. In the batch: 59 corrections in VolleyStation files, 4 in DataVolley files; the score errors left (26 in 52 VolleyStation files) are real rallies whose point code jumps the score (missed rallies, a set coded from 3:4).
- **Back-row attacks and blocks in the 2026-10-27 batch** (348 in 39 files) are ordinary front-row plays (`V4`, `X4`, quick `C1`/`X1`) by players the file's own columns 14–25 put in the back row (343 of 348 agree with our zones): the files' rotations drift from the court, not a rule error.
- **Checks from datavolley's `dv_validate` (2026-10-27).** Added, with counts on the 160-file corpus: a serve that won the point with no reception and no green code, not coded `#` (warning, 5 in 4 files); a second reception in one rally (warning, 20 in 9); the same player, skill and evaluation twice in a row (warning, 97 in 35, e.g. `*09AT-X6` twice); a set by the team that just received or dug, before the other team's attack (warning, 30 in 22); one player setting then blocking (warning, none); a front-row player attacking from a back-row zone (info, 228 in 68); a set on a `#`/`+` pass, or a setter call, by someone other than the setter on court (info, 160 in 34 and 153 in 19); a reception whose type or zones differ from its serve's (info, 269 in 10 and 349 in 8). Not taken, because the corpus shows them as ordinary play: a pass then an attack by the same player (800 in 84 files: sets often aren't coded), an attack's type against its set's and a block's or dig's against the attack's (3,300+), quicks by non-middles and middles' other balls (2,900, roles often missing or loose), a setter call on a poor pass (774). The team sheet's roles are now on `RosterPlayer` for later checks. Datavolley's "German" style checks (zone by role, P2/PP conventions) are a federation's conventions and aren't taken.
- What remains as errors looks like real scouting slips: missing point codes (second serve in one rally), skipped scores, missed rotations, wrong team on the point code.
- **Block pairing (task 7):** a blocked attack (`A/`) must be followed by the other team's block point (`B#`), and a `B#` preceded by the other team's `A/` (warnings, ServeReception). Both products check this; openvolley's `dv_validate` does not. Corpus: 6, all real slips (`a11AH/` followed by the opponent's set in C014; `a13AH+ a10BH#`, a block coded for the attacking team, in C001).
- **`R/` is not "opponent wins".** Another possible pairing check (`R/` only when the opponent won) would flag 22 corpus rallies that are fine: an overpass the serving team then attacks into the net or out (`*03SH/ a03RH/ *12AH=`, C001; also VolleyStation's C018). So it is only an Info hint ("check the evaluation"), agreed at the phase 2 review.
- **Knock-ons are Info.** An issue that follows from an earlier one is downgraded to Info with `cause` pointing at it (`courtside validate --info` prints "follows from line N"): after a point code given to the wrong team, the next rally's rotation issues and set-end issues that giving the point to the other team would fix; after a score error, a score issue on the next point code; after "won a side-out but didn't rotate", that team's rotation issues until the scout's next `z` correction. Rotation issues after a score-only error are not knock-ons (the score doesn't change who won).

### Bad-file set (task 7)

`crates/vb-engine/tests/bad_files` plants one slip per copy of a corpus .dvw (recipes in local `testdata/bad/recipes.toml`; the engine writes the file, so derived columns, `[3SET]` and `[3TEAMS]` are what DataVolley would have written) and compares Courtside with openvolley's `dv_validate` (validation level 3, `tools/parity/run.sh validate`). Terms in CONTEXT.md: a hit is a new Warning/Error between the slip's line and the next rally's serve; noise is any other new issue (issues already reported at the mapped line of the clean source don't count either way). datavolley 1.18.5, 30 files:

| Slip class | Courtside recall | dv_validate recall | Courtside noise | dv_validate noise |
|---|---|---|---|---|
| missing point code | 3/3 | 3/3 | 0 | 10 |
| skipped score | 3/3 | 3/3 | 0 | 7 |
| missed rotation | 3/3 | 3/3 | 0 | 4 |
| wrong-team point (rest of set renumbered) | 3/3 | 0/3 | 1 | 5 |
| 7th substitution | 3/3 | 0/3 | 0 | 0 |
| illegal re-entry | 3/3 | 0/3 | 0 | 0 |
| set ends 25:24 / 15:14 | 3/3 | 0/3 | 0 | 0 |
| libero attack | 3/3 | 3/3 | 0 | 0 |
| back-row block | 3/3 | 3/3 | 0 | 0 |
| `A/` written `A-` (opponent `B#` unpaired) | 3/3 | 0/3 | 0 | 0 |
| **Total** | **30/30** | **15/30** | **1** | **26** |

- Before the pairing check and knock-on suppression: Courtside 27/30, noise 22.
- `dv_validate` gets a point attributed against the touches only when the point code's score is also wrong ("Point assigned to incorrect team or scores incorrect"); a consistently renumbered set passes. Most of its noise is its level-3 style checks and "match result summary does not match the points recorded" after a missing point code.
- The one Courtside noise issue: wrong-team renumbering carries a set past 25 (set ends 28:21), which the knock-on rule doesn't trace back.

## Scout code grammar (vb-grammar)

- Skill code: team (`*`/`a`), player digits or `$$` (unknown), skill `S R E A B D F`, then 11 fixed positions (type, evaluation, combination ×2, target, start zone, end zone, end subzone, subtype, num players, special), each a character or `~` = empty, then free custom text from position 16. Trailing positions may be omitted (`a10AH#`) or written as `~` (`*19SM+~~~`); both round-trip.
- `$$&` is a "green code" (point credited or charged without a player touch; see "[3SCOUT] lines"). openvolley skips these entirely (no skill, player or evaluation), so parity compares only their team.
- Non-skill codes: `*p25:20` point (home:visiting after the point), `*z1` setter position, `*P08` setter player, `*c06:07` substitution (out:in), `*T` timeout, `**1set` set end. `>LUp` suffix on `P`/`z` marks the starting lineup.
- A line whose code starts with `>` is free text among the codes: challenges and referee notes (`>CHALLENGE…` C063 lines 482 and 980, `>REFEREE…` C054 line 684, `>CH…` C041 line 827, `>ROT<` in a doctored fixture from openvolley's tests). `Code::Comment`, a comment event in the fold (no part of a rally), written back verbatim.
- A full code's custom text starts at column 15 and may contain `.` (`*09EQ+K7C~4~2~~14F.+`, 39 codes in C044, 3 in C060); the command line only splits a dot code on a dot before that column.
- openvolley counts a setter-player code without `>LUp` (e.g. `aP11`, 196 in the seed corpus incl. LUp ones) as a substitution. Parity and the JSON export's `substitution` column mirror that; vb-engine treats it as a setter change (see Decisions).
- Seed corpus coding is shallow: 10,682 of 10,871 skill codes are 6 characters (`a10AH#`), the rest only add subtype and num players (`~~~~~~H2`). No combinations, zones or custom text yet, so the generated property tests carry that coverage until richer files arrive.

Seen once and unexplained (2026-10-27 batch): end subzone `E` in two DataVolley 4.02.34 files of one team (C161 line 171 `*07AQ#XF~21EH2…`, C162 line 211); DataVolley also let evaluation `C` and a player `X2` through in that batch, so likely typos. Player `00`, on no team sheet, in 20 codes of 10 VolleyStation files: maybe VolleyStation scouts' "unknown player"; reported as not on the team sheet until a file or the manual settles it.

## Scoresheets (eScoresheet PDF, `vb-scoresheet`, issue #57)

Observed on three local samples (2026-10-27, one league, eScoresheet releases 2026.10.02–04; `testdata/scoresheets/`, never committed).

- One landscape page (842 × 595), printed by "Synactis PDF In-The-Box". Every text item is its own `BT Tf Td Tj ET` in a WinAnsi TrueType font (Arial, Segoe UI; bold and italic from the base font's name), so a text run's position is its `Td`; one `q cm Q` wraps only the logo image. PDFKit and `lopdf` read it; `pdf-extract` 0.9 panics on it (its `Widths` arrays are two short of `FirstChar..LastChar`).
- Layout, found from fixed text, not order (row labels differ by release: "Rotation" / "Service Order", "Starting line up" / "No. of Starting player"): the title's two team names (bold, 12 pt), home left, each followed by its toss letter (A/B); per set a box located by its large bold set number; in each half the team name, `Start:`/`End:` times and `S`/`R` (served or received first) on one line, the Roman numerals I–VI 11 pt below, the starting six 8.5 pt below those; a substitute's number 14.2 pt below the six and the score at the substitution 25.6 pt below, a second score under it when the starter came back. Set 5 has a third half, "Points at Change", for the team changing ends; its substitutions after the change are written there. The points column's printed 1–25 is the form; points are crossed out with lines, so set scores come from the results table (T S W P | Set Time | P W S T, team A left; the winner's points bold).
- Substitution scores are the substituting team's own score first: checked against a match the user coded, every sheet score matched the coded score once read that way (the visitors' `11:14` is 14:11 home first).
- Team lists: a bold shirt number, then `LAST (N0000) First` (the licence number in brackets); liberos listed again under LIBERO with `L`; the captain's number in a rounded box, drawn as four corner curves (merged into one mark, ≤ 6 pt apart). Bench officials under "OFFICIALS" or "OFFICIAL MEMBERS ADMITTED ON THE BENCH": a role (`C` coach) then `Last (C0000) First` or `Last First`. Referees, scorer and assistant scorer to the right of their labels under APPROVAL.

## Disagreements with openvolley

- **Date serial is off by one in openvolley.** `read_match` converts the `[3MATCH]` line-2 serial as `as.Date("1900-01-01") + serial - 1`, which gives 2026-08-01 for `46234`. The correct epoch is 1899-12-30 (the spreadsheet convention): `46234` = 2026-07-31, matching the written date in all 17 DataVolley seed files (C001–C007, C008–C017). Courtside uses 1899-12-30. openvolley only uses the serial to disambiguate the written date, and its final `meta$match$date` equals `Dvw::match_date` on all 18 seed .dvw files (checked by hand against the pinned oracle), so the off-by-one never reaches its output.
- **Serve phase.** openvolley gives a serve `phase` "Reception" when it falls in the same team touch as a reception, which only happens when a point code is missing before it (C001 lines 75, 681, 719; C003 lines 544, 683, 1009; C006 line 273). A serve is always serve phase in Courtside. Everything else in openvolley's derived columns (set, scores, setter positions, rally and team-touch grouping, serving team, rally winner, winning attack, phase) matches on every touch (`crates/vb-engine/tests/parity.rs`). Courtside adopts openvolley's rule that a block straight after a reception-phase attack is in reception phase.

- **Team names.** openvolley trims them and, when both teams have the same name, appends " (home)" / " (visiting)" (C057, C040); the parity tests compare against the same rule (`openvolley_names`).
- **`$$` codes' custom text.** openvolley shifts it by three (`*$$FH=~~~~~~~~~-2`: custom text `-2` from column 15, openvolley `~~~-2`, C076 line 996) or invents it (`*$$BH!~~~~3~~2` ends at the blocker count, openvolley `~2`; C085 lines 311, 340, 478, 730, 888). Courtside follows the columns.
- **Reception end zone without a start zone** (`*10RH-~~~~2`, C110 line 105; C128 line 537 with subzone `C`): openvolley drops the end zone and subzone and gives the code as `*10RH-`; Courtside reads the columns.
- **Codes with a garbled player** (`aX2AH#…`, `*~1AH#…`, two DataVolley 4 files): openvolley still gives the team from the first character; Courtside leaves an unrecognised code without one.
- **`ROT` on a serve.** openvolley turns a serve with custom text `ROT` into a "Rotation error" skill and blanks its fields (C047 lines 186, 669). Courtside keeps it a serve with custom text `ROT`, which is what the code says.

## Decisions

Agreed at the phase 1 review (task 8).

- **One JSON surface (task 8, 2026-10-13):** the plays, stats and validation JSON live in `courtside-ffi`; the CLI and the WASM build both call it, and `tools/wasm/smoke.mjs` checks they agree on the corpus. The WASM functions take file bytes and return JSON strings (no serde-wasm-bindgen), so the web side parses one documented shape.
- **Phase 2 review (2026-10-13):** phase 2 accepted on the generated bad-file set; a blind re-test on unedited real scout files (labels from a hand check of every flagged line plus sampled rallies) follows in phase 3. Starting zones and set durations get regenerated as phase 3's first task. Issue kinds replace message matching in the knock-on pass when live validation arrives. The stateful WASM API waits for the bench viewer (phase 4).
- **Phase 2 validator gate (task 7, 2026-10-13):** on the bad-file set, Courtside's recall is at least `dv_validate`'s in every slip class and higher overall, and it adds no more noise. Recall alone would let a noisy validator win. `validator_beats_dv_validate` asserts it when the oracle output is current.

- **Setter code without `>LUp`** (`aP11`) is a change of setter in vb-engine and feeds rotation and setter-position tracking; it is not a substitution. The JSON export keeps openvolley's `substitution` flag for compatibility.
- **Lenient parsing.** The codec and grammar never reject a file: odd values are kept as written (`Code::Unrecognised`, `Role::Unknown`, unparsed dates) and `courtside validate` reports them. Rule checks belong to the validator and vb-engine, not the reader.
- **Dot codes fill the other half as VolleyStation does** (user, 2026-10-22, #22): when one half is rated and the other isn't, the other gets the season default if the rated half allows it, else the first rating the rated half allows. Changes from before: serve `-` → pass `+` (was `#`), attack `-` → dig `+` (was `#`). Correcting one half keeps the other's rating while it still fits. Serve `-` could give `#` or `+`).
- **A pass must fit its serve** (2026-10-24, #22): the validator warns (`PassDoesNotFitServe`) when the other team's reception straight after a serve has a rating its serve doesn't allow. Allowed: `S#`–`R=`, `S/`–`R/`, `S-`–`R#`/`R+`, `S+`–`R-`/`R!`, `S!`–`R!`/`R+`. DataVolley's compound table and VolleyStation's flow table agree on the first of each; `S+`–`R!` and `S!`–`R+` are a second convention in 20 of the 46 DataVolley 4 corpus files (621 pairs), so they pass. Left over in the corpus: about 150 pairs, nearly all in DataVolley 3 files (`S+`–`R+`, `S/`–`R#`).
- **Dig errors count against the digger** (user, 2026-10-21, #22): stats default to DataVolley 4's symbol sets, where a dig `=` is negative. VolleyStation's defaults leave it neutral (dig has no losing symbols); `--symbols volleystation` still gives that. An attack→dig dot code with a dig error, `12A.6d=`, writes `*12AH#  a06DH=` in all three products: the kill for the attacker, the dig error for the digger, one point.
- **No `winnow`.** The formats are positional; hand-written parsers are simpler. Revisit only for a real grammar.
- **Default and dot codes** are scouting-input features (expanding shorthand while typing) and are built with the scouting screen, not the file reader.
- **Corpus:** next priority is fully coded DataVolley files (combinations, zones, subzones, custom codes). Other code pages, beach and VolleyStation with video are wanted but not yet available.
