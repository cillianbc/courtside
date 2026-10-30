# Courtside is licensed under the AGPL-3.0

Courtside will be open source under the GNU Affero General Public License, version 3 only (`AGPL-3.0-only`, `LICENSE` at the root), for the whole codebase: the desktop app, the crates, the CLI and the WASM build. The paid offer is a hosted web version, run by us. We chose the AGPL over the GPL because its network clause makes anyone who runs a modified Courtside as a service publish their changes, which the GPL doesn't, so a hosted competitor can't take the code without giving back; and over a permissive licence (MIT, Apache-2.0) because those let anyone host it closed. "Only" rather than "or later" keeps a move to a future version a decision of ours. Decided by the user on 2026-10-19.

Consequences:

- **Our own hosting is unaffected:** as the copyright holder we can run Courtside as a service, and change its licence, without the AGPL's obligations binding us. That stays true only while we hold the copyright in all of it: before accepting outside contributions, adopt a contributor licence agreement (or a DCO plus an explicit licence grant to us) so contributed code can be hosted and relicensed too.
- **Binaries come with source.** Whoever gets a binary must be able to get its source. The CLI binaries are public today while the repo is private: publish the source, or hold CLI releases, before the next release under this licence.
- **Dependencies must be AGPL-compatible.** GPL-3.0, LGPL, MIT, Apache-2.0, BSD and ISC are; GPL-2.0-only is not. Check every new dependency, and add a `cargo deny` licence check before the first public release.
- **mpv and FFmpeg:** GPL builds of libmpv and FFmpeg are now usable (GPL-2.0-or-later and GPL-3.0 combine with the AGPL-3.0), including the common prebuilt ones, and the `libmpv2` crates' LGPL-2.1 is compatible. Shipping them means shipping their licence texts and their source (or a written offer for it) with each release. See ADR 0004 and docs/THIRD-PARTY.md.
- **The corpus is not part of the code.** `testdata/` stays private and never ships.
