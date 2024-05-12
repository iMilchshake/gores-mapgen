# Procedual Map Generator for KoG

Procedual random map generator for the _gores_ gamemode in teeworlds/ddnet (see [KoG](https://kog.tw/)). This is a rust rewrite™ of my previous Unity3D based [generator](https://github.com/iMilchshake/gores-map-generation). This project is in active development, many parts of the codebase might still change. You can playtest generated maps on my ddnet server, just search for "random" in the server browser.


### Screenshots

**Editor**:
![](https://github.com/iMilchshake/gores-mapgen-rust/blob/main/docs/editor_preview.png?raw=true)

**Ingame**:
![](https://github.com/iMilchshake/gores-mapgen-rust/blob/main/docs/ingame_preview.png?raw=true)


### Usage
Assuming that you have [rust installed](https://rustup.rs/) just `git clone` and then run `cargo run` inside the project directory. For documentation on all the possible settings check out the docstrings for the `GenerationConfig` struct in `config.rs`.

### Keybinds
`e`: Export map

`space`: Generate map

`r`: Refocus camera

