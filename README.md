# Procedual Map Generator for KoG

Procedual random map generator for _gores_ DDNet maps (see [DDNet Wiki](https://wiki.ddnet.org/wiki/Gores), [KoG](https://kog.tw/)). This project is in active development, many parts of the codebase might still change. You can playtest maps on my ddnet server, just search for "random" in the server browser.

## Screenshots

![](https://github.com/iMilchshake/gores-mapgen-rust/blob/main/docs/ingame_preview.png?raw=true)

## Editor Usage
Assuming that you have rust installed just `git clone` and `cargo run` inside the project directory. Alternatively, pre-compiled binaries of stable versions are provided as GitHub releases.
For documentation on all the possible settings check out the docstrings for the `GenerationConfig` struct in `config.rs`.

**Keybinds**:
- `space`: Generate map
- `shift+space`: Generate map (retry on failure)
- `r`: Refocus camera
- `d`: View debug layer hover

## WASM / Web Editor

The editor can run in your browser via WebAssembly! Try it at: **[Live Demo](https://iMilchshake.github.io/gores-mapgen/)**

## Snapshot Testing
Snapshot tests ensure code changes don't unintentionally alter map generation. For comfortable interactive usage `cargo install cargo-insta`.

**Run tests:** `cargo insta test`
**Review changes:** `cargo insta review`
**Accept all:** `cargo insta accept`
