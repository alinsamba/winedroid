# Third-party notices

These notices cover third-party material included in this repository.
Cargo dependencies retain their own licenses; distributions of native binaries
or the WebAssembly demo must also include the notices required by their bundled
components.

## Roboto fonts

`assets/fonts/Roboto-Regular.ttf` and `assets/fonts/Roboto-Bold.ttf` are licensed
under the Apache License, Version 2.0.

The font files contain: Copyright 2011 Google Inc. All Rights Reserved.
The accompanying attribution states: Copyright 2011 The Roboto Project Authors.

See the [font attribution](assets/fonts/LICENSE.txt) and the
[full Apache-2.0 license](assets/fonts/LICENSE-APACHE.txt).

## FxHash

`src/fxhash.rs` adapts the FxHash mixing scheme from
[`rustc-hash` 1.1.0, `src/lib.rs`](https://github.com/rust-lang/rustc-hash/blob/6fc9a706a9312ee801c7e77ca06c0782fa9eadef/src/lib.rs).
Our implementation uses a fixed 64-bit state, little-endian byte chunks, and
zero-padded tail processing.

We use the upstream code under its MIT license, reproduced below.

Copyright 2015 The Rust Project Developers.

```text
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```
