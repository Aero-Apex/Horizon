# Third-party notices

## CullLeaves

The optional leaf and mangrove-root face-culling behavior is adapted from
[CullLeaves](https://github.com/TeamMidnightDust/CullLeaves) by MidnightDust.
Its MIT license is reproduced below.

```text
MIT License

Copyright (c) 2020 MidnightDust

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## wgpu-hal

`third_party/wgpu-hal` is [wgpu-hal](https://github.com/gfx-rs/wgpu) 30.0.1 by the gfx-rs
developers, with one upstream fix applied
([gfx-rs/wgpu#10124](https://github.com/gfx-rs/wgpu/pull/10124)). It is licensed under MIT or
Apache-2.0; both licenses are in that folder (`LICENSE.MIT`, `LICENSE.APACHE`).

## Pixelify Sans

`crates/horizon-theme/fonts/PixelifySans-*.ttf` is [Pixelify Sans](https://github.com/eifetx/Pixelify-Sans)
by The Pixelify Sans Project Authors, licensed under the SIL Open Font License 1.1
(`crates/horizon-theme/fonts/OFL-PixelifySans.txt`).

## Pixelarticons

`crates/horizon-theme/icons/pixel/*.svg` are icons from
[Pixelarticons](https://github.com/halfmage/pixelarticons) by Gerrit Halfmann, under the MIT
license (`crates/horizon-theme/icons/pixel/LICENSE-pixelarticons.txt`).

## Continuity

`crates/horizon-client/src/ctm.rs` is a Rust rewrite of the connected-texture logic of
[Continuity](https://github.com/PepperCode1/Continuity) by PepperCode1 and its contributors,
licensed under the GNU Lesser General Public License 3.0: the 47-tile index table, the direction
maps, the `horizontal`, `vertical`, `horizontal+vertical`, `vertical+horizontal`, `top`, `random`,
`repeat` and `fixed` methods, the random mixing and the `.properties` parsing follow its Java
source (`processor/simple/*`, `processor/DirectionMaps`, `properties/*`, `util/QuadUtil`,
`util/MathUtil`). This file is derived from that work and is offered under the same license;
the LGPL-3.0 text is at <https://www.gnu.org/licenses/lgpl-3.0.html>.

## AppleSkin

`crates/horizon-client/src/appleskin.rs` and `game/appleskin.rs` port
[AppleSkin](https://github.com/squeek502/AppleSkin) by squeek502 (the Unlicense, public domain):
the HUD overlays, the health estimate, the tooltip row and the F3 line. The two images in
`crates/horizon-client/assets/appleskin/` (`icons.png`, `tooltip_hunger_outline.png`) are AppleSkin's
own, public domain.

## Controlling and Searchables

`crates/horizon-client/src/controlling.rs` and the Key Binds page in `options_ui.rs` port
[Controlling](https://github.com/Jaredlll08/Controlling) and [Searchables](https://github.com/Jaredlll08/Searchables)
by Jared Lloyd (Jaredlll08), both MIT licensed: the search language, the filters, the sort orders and
the free keys list.

## Jade

The health and armor rows and the default colours of the Jade overlay (`jade.rs`, `game/jade.rs`) follow
[Jade](https://github.com/Snownee/Jade) by Snownee (`HealthElement`, `ArmorElement`, `ColorPalette`,
the `jade:dark` theme), which is licensed under Creative Commons Attribution-NonCommercial-ShareAlike 4.0
(<https://creativecommons.org/licenses/by-nc-sa/4.0/>). That part of Horizon is therefore for
non-commercial use, and changes to it must be shared under the same license.
