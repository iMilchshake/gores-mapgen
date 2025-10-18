# Change Log

## 0.12.4

#### Compression Level 9 (from 6)

Size, load time and load+save time comparison for all DDNet brutal maps.
As maps are typically loaded/distributed many more times than saved, this version tries out the max compression level 9.

```
         size  load  load+save
zlib     231M  34s   174s
zlib-ng6 238M  12s    67s
zlib-ng7 225M         89s
zlib-ng8 216M        119s
zlib-ng9 209M        186s
```

#### Add deprecation notes to some parse/save functions

There are currently many many parse functions and also multiple save functions.
In the future, the functions which are specialized to only work on files will be removed.
Instead, the more generic parse/save functions should be used.

(The parse function currently operates on `&[u8]`, maybe it should take a `Read` instead)

## 0.12.3

#### Compression Backend Change

The measured speed-up in pure map loading is x3! 🚀

Downside is that newly saved map files will no longer be bit-to-bit equivalent to maps saved in DDNet.

`zlib-ng` is now the compression backend for `flate2`. Previously we used `zlib`.
We use the feature `zlib-ng-compat`, which falls back to `zlib` for cases like WebAssembly.

#### Proper Support for Empty Quads & Sounds Layers

See https://github.com/ddnet/ddnet/pull/8102 for more details.

#### Minor fixes

- Removed a `println!` in automapper code.
- Minor changes to log messages

## 0.12.2 and older versions

TODO
