# Third-party notices

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

## Zoomify
The zoom's easing and scrolling (`zoomify.rs`) are a Rust rewrite of
[Zoomify](https://github.com/isXander/Zoomify) by isXander (`ZoomHelper`, `Interpolator`,
`TransitionType`, `DefaultZoomHelpers`, and the mouse and camera mixins' formulas), under the GNU Lesser
General Public License 3.0 or later. Its source is at the link; this project's changes are the rewrite itself.

## Mouse Tweaks
The inventory wheel (`mouse_tweaks.rs`) and the drags in `game/tweaks.rs` follow
[Mouse Tweaks](https://github.com/YaLTeR/MouseTweaks) by Ivan Molodetskikh (YaLTeR) (`Main.onMouseScrolled`, `findPullSlot`,
`findPushSlots`, `Config`), under the BSD 3-Clause License:

> Copyright (c) 2023, Ivan Molodetskikh. All rights reserved. Redistribution and use in source and binary
> forms, with or without modification, are permitted provided that the copyright notice, the list of
> conditions and the disclaimer of the BSD 3-Clause License are kept; the names of the copyright holder and contributors may not be
> used to endorse derived products without permission. THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY.

## LambDynamicLights
The dynamic light rules (`dynamic_light.rs`, `game/dynamic_lights.rs`: the item and entity table, the falloff
and its 7.75 block radius) follow [LambDynamicLights](https://github.com/LambdAurora/LambDynamicLights) by
LambdAurora, whose source is under **The Lambda License**. That license allows sharing and changing the
source with the original authors listed, but **sharing binaries needs the authors' written approval**.
Builds containing this port are for private use until that approval is given. Original:
https://github.com/LambdAurora/LambDynamicLights/blob/26.3/LICENSE

## Entity Model Features
Custom entity models (`cem.rs`, `cem/expr.rs`, `cem/jem.rs`) read OptiFine's CEM format (`.jem`, `.jpm`,
`.properties`, and the animation expressions) as [Entity Model Features](https://github.com/Traben-0/Entity_Model_Features)
by Traben reads it: how a part hangs from the entity's own model part, the axis flips and box layouts, the
expression grammar and functions, the variables, and the OptiFine-to-vanilla part names in
`assets/cem/optifine_parts.json` (from its `EMFModelMappings`, for Minecraft 26.3). EMF is licensed under the GNU Lesser
General Public License 3.0 or later; its source is at the link, and this project's changes are the Rust rewrite.
