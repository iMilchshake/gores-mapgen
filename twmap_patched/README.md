TwMap
===

Safely parse, edit and save [Teeworlds](https://www.teeworlds.com/) and [DDNet](https://ddnet.org/) maps.

Also check out: [TwGpu (Renderer)](https://gitlab.com/Patiga/twgpu), [Python bindings](https://gitlab.com/Patiga/twmap-py), [Blender Addon](https://gitlab.com/Patiga/twmap-blender/)

Goals of this library:

- Performance
- Reasonable map standards
- Compatibility with all [Teeworlds (0.7) maps](https://github.com/teeworlds/teeworlds-maps) and [DDNet (0.6) maps](https://github.com/ddnet/ddnet-maps)

The binary map format is documented in the libtw2 repo [here](https://github.com/heinrich5991/libtw2/blob/master/doc/map.md).

Note that the library does support compilation to [webassembly](https://www.rust-lang.org/what/wasm).
Since version `0.10.0`, saving maps in webassembly should produce the same binary output.

Usage
---

- Add `twmap = 0.12.4` in your `Cargo.toml`
- Check out the [docs](https://docs.rs/twmap/latest/twmap/)

Supported map formats
---

- DDNet (0.6)
- Vanilla (0.7)
- MapDir
    - folder/directory based format
    - embedded images as .png files
    - embedded sounds as .opus files
    - all other parts of the struct as .json files

Compression
---

Parts of the binary map data are compressed with zlib compression, this is the main slowdown factor when loading maps.
To compensate for this, the library will not load layer, image and sound data on parsing, instead this can be done manually.

If you want to save the map again, then the library will decompress everything anyways (its included in the `save` method).
However, if that is not the case, and you only want to read some part of it, you can hold off on performing the `load` method on the map struct.
Instead, perform `load` only on the compressed parts of the map that you want to read.

Note that some methods provided by the library require parts of it to be loaded properly. Keep that in mind while playing around with partially unloaded maps.

Checks
---

This library has a lot of constraints on its map struct that it enforces on saving maps.
For instance every map must have a Game group, Game layer, layer names must be at most 11 bytes, etc.
All constraints are put into place to ensure proper saving and parsing, meaning that the library guarantees you well-behaved maps.

Note that many methods provided by the library require certain constraints to be fulfilled.
Keep that in mind when using `parse_unchecked` instead of `parse`, which will leave out most of the checks.
