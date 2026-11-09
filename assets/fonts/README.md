# Embedded fonts

`Righteous-Regular.ttf` is the display face used by the world-space lyrics
(`client/src/graphic/fonts.rs`). It is embedded into `libclient` at compile
time.

- **Font:** Righteous
- **Designer:** Brian J. Bonislawsky (Astigmatic / AOETI)
- **Source:** <https://github.com/google/fonts/tree/main/ofl/righteous>
- **License:** SIL Open Font License 1.1 — see `Righteous-OFL.txt`

The rest of the client keeps egui's theme font; Righteous is registered as a
separate named family so only the lyric billboards opt into it.
