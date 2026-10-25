# Custom entity models (OptiFine's CEM)

A resource pack can replace mob models with `assets/minecraft/optifine/cem/<model>.jem` files and animate them
with expressions, as Fresh Animations does. Horizon reads them in `cem.rs` (the model and its run time),
`cem/jem.rs` (the files) and `cem/expr.rs` (the expressions). Credits are in `THIRD_PARTY_NOTICES.md`.

## What a model is

A `.jem` file lists parts. Each names a part of the game's own model for the mob (`"part": "head"`) and
**hangs from it**: the game's part keeps its pose from the mob's own animation, the pack's part is its child, and
the game's boxes are dropped unless the pack part says `"attach": true`. A `.jpm` file named by `"model"` fills in
what the part leaves out (Fresh Animations keeps its animations there).

- OptiFine's part names (`leg1`) are mapped to the game's (`right_hind_leg`) with `assets/cem/optifine_parts.json`,
  generated from Entity Model Features' tables for 26.3 by `tools/extract_cem_parts.py`.
- `invertAxis` flips the axes of `translate`, `rotate` and the boxes; boxes are a `textureOffset` (the usual
  cuboid layout, `mirrorTexture` flips it) or one rectangle per face (`uvNorth`, `uvUp`, ...), grown by `sizeAdd`.
- The model file for a mob is the model's name: `pig`, `pig_baby`, `warm_pig`, `sheep_wool` (the model layer
  `sheep#wool`). A `.properties` file next to it picks `rule_index` by the blocks a mob stands in or on
  (`blocks.N`) and a few NBT flags (`nbt.N`).

## Animations

Every frame, in the order they are written (grouped by the part they belong to), the lines run:
`body.rx = ...` sets a part's pose (`tx ty tz rx ry rz sx sy sz`, in pixels and radians, `visible`,
`visible_boxes`), `var.x = ...` / `varb.x = ...` set a number or a boolean kept for the mob from frame to frame.
Names in an expression are the mob's state (`limb_speed`, `age`, `head_pitch`, `health`, `is_in_water`, ...), a
part's current pose (`right_arm.rx`, which still holds the game's own animation before a line writes it) and the
variables. Booleans are `+inf` and `-inf`; a line that gives NaN changes nothing. A line that does not parse
turns the model's animations off and is logged.

## Not done

Part textures (`texture` on a part), `sprites`, attachment points, `baseId`, armor layer models, the `render.*`
outputs (shadow size, leash), and the properties `biomes`, `heights`, `health`, `name` and the like (a rule using one
never matches).

## Testing

`cargo test -p horizon-client --lib cem` runs the unit tests. With the prepared resources and
`resourcepacks/Fresh Animations 1.11.1.zip` present, `-- --ignored` also builds every model of the pack over the
game's models (`every_model_of_the_pack_builds`), prints the poses animations give at rest
(`poses_of_mobs_at_rest`), and compares the size of every drawn mob with and without the pack
(`drawn_sizes_with_and_without_the_pack`).
