# .dvw / .vsm format notes

Everything learned about the formats, each entry with the corpus file(s) that show it.
When Courtside and openvolley disagree, check the file by hand, decide who is right, and record it here.

File references are relative to `testdata/`. `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `dvw/datavolley4/corpus-file`, `corpus-file` = `corpus-file`.

## Sections

- All 18 seed files have the same 14 sections in the same order: `3DATAVOLLEYSCOUT`, `3MATCH`, `3TEAMS`, `3MORE`, `3COMMENTS`, `3SET`, `3PLAYERS-H`, `3PLAYERS-V`, `3ATTACKCOMBINATION`, `3SETTERCALL`, `3WINNINGSYMBOLS`, `3RESERVE`, `3VIDEO`, `3SCOUT`. No lines before the first header. (all)
- Empty sections are just the header line followed by the next header (`3ATTACKCOMBINATION`, `3SETTERCALL`, `3RESERVE`, `3VIDEO` in corpus-file, corpus-file).
- `[3SET]` always has 5 rows; unplayed sets are `True;;;;;;`. Partial scores may be space-padded: `8 -7` (corpus-file), ` 7- 8` (corpus-file).
- `[3VIDEO]` line is `Camera0=<absolute local path>` (corpus-file). That path is personal data; don't echo it into exports by default.

## Header ([3DATAVOLLEYSCOUT])

- `FILEFORMAT: 2.0` in all files.
- `GENERATOR-PRG: Data Volley` / `GENERATOR-REL: Release 4.2023.03` (corpus-file) and `Release 4.2025.01` (corpus-file), `GENERATOR-VER: Media`.
- VolleyStation writes `GENERATOR-PRG: VolleyStation`, `GENERATOR-VER: Professional`, empty `REL` and `NAM`, keeping the trailing space after the colon (`GENERATOR-REL: `). Must round-trip that space. (corpus-file)
- Date format differs by generator: DataVolley `MM/DD/YYYY` (corpus-file, corpus-file), VolleyStation `DD/MM/YYYY` (corpus-file). Same in `[3MATCH]` field 1. Ambiguous for day ≤ 12; decide by `GENERATOR-PRG`, not by guessing.

## Encodings

- `[3MATCH]` field 9 is the Windows code page: `1252` in all files.
- 13 files are pure ASCII. 5 contain raw cp1252 bytes in player names, which are invalid UTF-8: `í` = 0xED (corpus-file), `ò` = 0xF2 (corpus-file, -05, -08, -10). Tools that sniff UTF-8 (grep, `file`) treat these as binary.
- DataVolley also stores names hex-encoded in trailing columns, prefixed with `2`: `Siobhán` → `253696F6268E16E` (corpus-file), i.e. `2` + hex of the cp1252 bytes. Seen in `[3MATCH]`, `[3TEAMS]`, `[3MORE]`, `[3PLAYERS-*]`. VolleyStation omits these columns (corpus-file).
- Tokeniser works on bytes, so encoding is irrelevant to round-trip; decoding is task 4.

## Line endings

- DataVolley: CRLF throughout, trailing CRLF at EOF (corpus-file, corpus-file).
- VolleyStation: LF throughout, **no** trailing newline at EOF (corpus-file).
- Tokeniser stores each line's terminator, so both round-trip byte-exact.

## Field content

- Literal tab characters inside fields: `Surname Forename<TAB>` in `[3TEAMS]`, `Forename<TAB>` in `[3PLAYERS-H]` (corpus-file). Never trim fields.
- Competition names can contain `|` (corpus-file: `CEV U20 ... 2024 | Men - Pool in IRL`).
- `[3PLAYERS-*]` column 15 is empty in most DataVolley rows and `False` in some (corpus-file visitors) and all VolleyStation rows.

## [3SCOUT] lines

- Each line: code, then `;`-separated metadata, then 12 on-court player numbers (6 home, 6 away).
- DataVolley Media files have no video column values (no video). VolleyStation fills a video-seconds column (`236`, `244`, … in corpus-file).
- Set end line: `**3set;...` with on-court columns empty (corpus-file, corpus-file).
- Unknown-player attacks/blocks coded `$$&H#` (team prefix `*` or `a`).

## Generator quirks

## Disagreements with openvolley
