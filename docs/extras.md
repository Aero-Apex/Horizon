# Horizon extras

Optional conveniences built into the client: fullbright, dynamic lights, zoom, a minimap, an FPS counter, a
keystroke overlay, auto-reconnect, Mouse Tweaks-style inventory controls and
inventory sorting. **Every one is off by default**: a fresh
install looks and behaves exactly like vanilla 26.3, and turning an extra off
again restores vanilla's behaviour completely.

## Where they live

| Part | Code |
|---|---|
| Settings, parsing and saving | `crates/horizon-client/src/extras.rs` |
| Zoom curve, FPS and click counters | `extras.rs` (pure logic, unit tested) |
| Dynamic lights: item glow, nearest sources | `crates/horizon-client/src/dynamic_light.rs` (unit tested), `game/dynamic_lights.rs`, `terrain.wgsl` and `entity.wgsl` (`dynamic_block_light`) |
| HUD drawing (FPS, keystrokes, minimap frame) | `crates/horizon-client/src/extras_hud.rs` |
| Minimap colours, tiles and image | `crates/horizon-client/src/minimap.rs` |
| Reconnect policy and backoff | `crates/horizon-client/src/reconnect.rs` |
| Game glue (input hooks, per-frame update) | `crates/horizon-client/src/game/extras.rs` |
| Mouse Tweaks-style controls, sort buttons | `crates/horizon-client/src/game/tweaks.rs` |
| Sort order and its clicks | `crates/horizon-client/src/inv_sort.rs` (pure, unit tested) |
| The settings page | `options_ui.rs` (`Page::Extras`): **Options → Horizon Settings... → Extras...** |

The settings are grouped by extra, under a header each, two buttons to a row as
vanilla's options lists are, and each says what it does in its tooltip. An
extra's own settings (its key, corner, size...) are greyed out while its switch
is off (`Item::parent`).

`game.rs` only gets one-line hooks (key, wheel, mouse, frame, HUD), so the
extras stay out of the way of other input and options work.

## Options storage

`horizon-extras.txt` beside `options.txt`, in the same `key:value` format as
`horizon-graphics.txt`. `options.txt` is untouched, so a vanilla client can share
the folder. Unknown or broken lines fall back to the defaults.

The zoom key is a Horizon setting stored in that file (`zoomKey:key.keyboard.z`)
and bound on the Extras page, not in vanilla's Key Binds screen: Horizon has no
Horizon category in Controls, and adding one would put a non-vanilla mapping into
`options.txt`.

## The extras

### Fullbright (`fullbright`, default off)
- Displays blocks and entities with a fully bright lightmap, including in dark caves.
- Turning it off restores the normal lightmap using the current brightness,
  time of day and effects. The brightness slider keeps its normal range.

### Dynamic lights (`dynamicLights`, default off)
A port of LambdAurora's LambDynamicLights' rules, drawn by this client's own renderer. Not
in vanilla.
- **What lights**: the player and every mob, by what they hold or wear in any equipment slot (main
  hand, off hand, head, chest, legs, feet) and by what they are; a glowing or burning mob gives 15;
  items dropped on the ground; minecarts with a lit display block (a lit furnace cart); item frames
  (a glow item frame 12, any frame its item's light); fireballs, wither skulls and arrows (spectral
  ones glow, burning ones give 15); falling glowing blocks; primed TNT and swelling creepers, as their
  own settings say; anything on fire (15). A spectator gives nothing.
- **Creepers** (`dynamicLightsCreeper`, default simple): 10 while swelling, or its swell in tenths
  (fancy), or off. **TNT** (`dynamicLightsTnt`, default off, as LambDynamicLights has it): 10 (simple),
  or fancy: none while the fuse is full, growing to 10 as it burns down (the mod's smoothstep).
- **Items**: the mod's own table first: blaze powder and rods 10, fire charges 10, glow berries 14,
  glow ink sacs 12, glowstone dust 10, nether stars 8, prismarine crystals 10, spectral arrows 8, lava
  buckets 15; torches and campfires glow like their block. Every other block item glows like its block's
  default state (`getLightEmission`): lanterns, glowstone, sea lanterns 15.
  Items that need air (torches, campfires, fire charges, lava buckets) go dark when the holder's eyes are in water.
- **Mobs by type**: allays 8, blazes 10 (4 in water), magma cubes 8 or 11 when squashed past 0.6,
  glow squid 12, an enderman the light of the block it carries.
- **Falloff** is the engine's: `level - distance / 7.75 * 15`, and nothing past 7.75 blocks
  (`SpatialLookupEntry.lightAtPos`). Entities light from their eyes.
- Client-only, at render time: the light engine, meshes, saves and other players never see it.
  Each frame the game picks the 16 sources nearest the camera (`dynamic_light::select`);
  terrain.wgsl and entity.wgsl raise each vertex's block light to the source's level when that is
  brighter. The light doesn't stop at walls.
- Under a Java shader pack the lights reach the pack's terrain too: the glue that rebuilds the lightmap
  coordinates (`gfx/java_terrain.rs`, `hz_dynamic_light`) reads the same 16 lights from its table, so the
  pack's own lighting sees them as block light. Entities take them from entity.wgsl as always.
- Not done: display entities (block and item displays, which LambDynamicLights lights by their
  block or item and brightness override): Horizon has no display entities yet. Glow squid dark ticks
  (it stays at 12). Throwable items (eggs, pearls, potions), which glow by their item only.
  The mod's data pack format, and its chunk rebuilds (the light is applied per vertex, so none happen).
- Off: nothing is gathered and the shaders' loop runs zero times (the count is 0), so it costs
  nothing measurable.
- Logic and tests: `dynamic_light.rs`; the game side `game/dynamic_lights.rs`.

### Zoom (`zoom`, default off)
A port of isXander's Zoomify (LGPL-3.0-or-later): its `ZoomHelper`, interpolators, transition curves and
spyglass rules (`zoomify.rs`), with the mod's defaults.
- **Zoom key** (default **Z**: C is vanilla's `key.saveToolbarActivator`): hold it to zoom, or set
  "Zoom key behaviour" to Toggle. The key does nothing in chat, signs, books, text fields and every other
  screen, and is never taken from vanilla's mappings (the Extras page marks it "Also bound").
- **Initial zoom** 4x, **zoom in** 1 s and **zoom out** 0.5 s along the **ease out exponential** curve; all
  14 of the mod's curves are there, in and out separately (re-pressing mid-way continues from where the
  picture is, as the mod's `inverse` does).
- **Scroll zoom**: the wheel while zooming takes one of ten steps further in or out, each multiplying
  the zoom by 150% (the geometric `zoomPerStep`), eased by the mod's smoothness 70; scrolling out stops
  at normal field of view. Steps are forgotten when the key is let go, unless "Keep scroll steps" is on.
- **Zoom in and out keys** (`zoomInKey`, `zoomOutKey`, unbound by default) step the zoom instead of the
  wheel when "Zoom steps with keys" is on, as Zoomify's keybind scrolling does; the wheel then turns the hotbar.
- The field of view is divided by the zoom (limited to 1x to 500x), the hand too ("Zoom the hand too"),
  the mouse turns by the sensitivity divided by `lerp(relative sensitivity, 1, zoom)`, and view bobbing is
  divided by `lerp(0.2, 1, zoom)`.
- **Cinematic camera** (`zoomCinematic`, 0 to 100%, default 0): while zooming, the mouse is smoothed and
  its smoothing divided by the percentage, as Zoomify does. The secondary zoom smooths the mouse too.
- **Secondary zoom** (default key **F6**): a second, slow zoom that eases in over 10 s to 4x and out over 1 s,
  on top of the first. It hides the HUD while on, unless "Hide HUD on secondary zoom" is off.
- **Spyglass** (`zoomSpyglass`, default combine): combined with the zoom, the spyglass zooms as vanilla
  does and the zoom key adds to it; "Override" makes scoping zoom by the zoom instead of the spyglass's
  own FOV and sensitivity; "Only while holding" and "Only while carrying" let the zoom key work only with
  a spyglass in hand or in the inventory (scoping always zooms).
- **Spyglass overlay** (`zoomOverlay`, default holding): when the scope is drawn: never, while holding or
  carrying one, or always while zooming. **Spyglass sounds** (`zoomSpyglassSound`, default with the
  overlay): the use and stop sounds play never, always, only with a spyglass, or with the overlay.
- Older `horizon-extras.txt` files load: `zoomSmooth:false` becomes instant transitions and
  `zoomSensitivity:false` a relative sensitivity of 0.
- With zoom off (or the key up and no steps), the divisor is exactly 1 and the field of view, hand,
  mouse and bobbing are vanilla's, bit for bit.

### FPS counter (`fps`, default off)
- A small "N fps" line in a corner (default top left), optionally followed by
  the minimum and the 1% low over the last 5 seconds.
- Separate from the Advanced graphics "Overlay" (the frame-time graph).

### Keystrokes (`keystrokes`, default off)
- W A S D, Space, LMB and RMB keys that light up while held, with clicks per
  second under the mouse buttons (clicks in the last second). The letters follow
  the bound keys.

### Minimap (`minimap`, default off)
- A corner map (top right by default), square or round, small/medium/large,
  zoom 0.5 to 4 blocks per pixel, rotating with the player or north up, with the
  player arrow (vanilla's map decoration sprite).
- Colours are vanilla's `MapColor` per block, as map items use, with the map
  item's height shading and water-depth dithering.
- Underground, in the Nether and under any roof, it maps the floor around the
  player's height instead of the top block, so caves read sensibly.
- Cost: one cached 16x16 tile per loaded chunk (packed map colour and height).
  Tiles are filled nearest first and refreshed when blocks change, within a time
  budget of 0.3 ms a frame; composing the 128x128 image is a few thousand
  lookups, done only when the view moved or a tile changed, and uploaded into a
  dedicated atlas slot. With the minimap off nothing runs.

### Auto-reconnect (`autoReconnect`, default off)
- On the "Connection lost" screen after a multiplayer connection drops, a
  countdown "Reconnecting in N s" with a Cancel button, then a new connection
  to the same address.
- Backoff 5, 10, 20, 40, 60 s; at most 5 attempts, then it stops. The count
  only resets after staying connected for a minute, so a server that drops
  players at once never causes an endless loop.
- Never after a kick, a ban, a whitelist or version refusal, a duplicate login,
  an online-mode refusal (no account signed in), or a session refusal from the
  account's login (invalid session, banned from playing online, multiplayer
  disabled, the server failing to verify the username). Unreachable session
  servers, on either side, are retried with the same backoff.
- Online-mode servers: the retry joins with the account the game signed in
  with, held in memory for the whole run (`auth::current()`); nothing is
  written to disk.

### Mouse Tweaks controls (`mouseTweaks`, default off; `mouseTweaksWheel`, default on)
A port of YaLTeR's Mouse Tweaks (BSD 3-clause): the drags (`Main.onMouseDragged`) and the wheel
(`Main.onMouseScrolled`, planned in `mouse_tweaks.rs`). In the player inventory and crafting, chests and
their kin (barrels, ender chests, shulker boxes, hoppers, dispensers, droppers), furnaces (smelting,
blasting, smoking), brewing stands, crafters, and the station screens (anvil, smithing, grindstone,
stonecutter, loom, cartography table, enchanting table, beacon, villager trades), in singleplayer and on
servers (the server's menus are predicted the same way; untested on a server):
- Right-drag holding a stack (`mouseTweaksRmb`): one item goes into each slot entered, at once
  (slots holding another item, or full, are skipped).
- Left-drag holding a stack (`mouseTweaksLmbItem`): each slot entered that holds the same item is
  gathered into the hand, when it all fits. This takes over only while vanilla's drag covers one slot,
  so a left-drag across empty slots still splits the stack as vanilla does.
- Shift + left-drag with an empty hand (`mouseTweaksLmbEmpty`): every slot entered is quick-moved;
  holding a stack, only the slots of its item.
- The wheel over a slot (`mouseTweaksWheel`) is the mod's: down moves items of it to the other side (the
  container, or the crafting grid, or a furnace's input and fuel as the menu allows), up brings items of
  the same kind back. It works with a stack in hand (put back after), moves as many items as it scrolled
  (or one: "Wheel items per tick"), looks for items in the other inventory from its last slot (or first:
  "Wheel pulls from"), and can be inverted or follow where the other inventory is on screen ("Wheel
  direction"). Over a slot the wheel is always the tweak's, even when nothing moves. Results are only
  taken from (a crafting or smelting batch at a time), never put into, and the menu's own rules decide
  where an item may go (`Menu::slot_rules`).

Every action is ordinary slot clicks (`ContainerClick` on a server), so a
vanilla server sees nothing unusual. With the extra off, the hooks return at
once and the screens click exactly as vanilla's (`tools/test_menu_click_parity.py`
and the container screen tests are unchanged). Not done: the armour and off-hand slots are not in the
wheel's reach, the horse's inventory, and the mod's ignore and disable annotations and API.

### Sorting (`sort`, default off; `sortKey`, unbound)
- A small button right of the container's title (chests, barrels, ender
  chests, shulker boxes) and one right of the "Inventory" line (the main
  inventory; the hotbar is left alone). The key, when bound on the Extras
  page, sorts the part under the cursor (the container when over no slot).
- Partial stacks of one item merge, then the stacks go in creative tab order
  (the search tab's), then by name; full stacks before the partial one,
  empty slots last. It needs an empty hand.
- Done with ordinary left clicks planned by `inv_sort::plan`, so a vanilla
  server accepts it; a unit test replays the planned clicks through the menu
  engine on random chests.

### AppleSkin (`appleSkin`, default off)
A port of squeek502's AppleSkin (public domain), drawn on this client's own HUD
with the mod's own `icons.png`. Sub-switches (all on with the extra):
`appleSkinRestored`, `appleSkinSaturation`, `appleSkinExhaustion`,
`appleSkinHealth`, `appleSkinTooltips`, `appleSkinTooltipsAlways`,
`appleSkinOffhand`, `appleSkinDebug`.
- **Saturation**: golden icons from the sheet over the food icons, in the mod's
  four sizes (full, three quarters, a half, a quarter).
- **Exhaustion**: the sheet's strip behind the food bar, growing from the right
  as exhaustion builds to 4, 75% opaque.
- **Food in hand** (main hand, else the off hand; only food the player could
  eat, `Player.canEat`): the hunger it restores flashes over a faint empty
  icon (green icons for food with a harmful effect), the saturation it adds
  flashes as golden icons, and the hearts it would heal flash over the health
  bar. The pulse is the mod's: a triangle wave stepping 1/8 a client tick
  between -0.5 and 1.5, clamped, at most 0.65, restarting when no food is held.
- **Health estimate** is the mod's `getEstimatedHealthIncrement`: natural
  regeneration from the food after eating (fast regeneration in one step) plus
  the heal of a Regeneration the food applies. It is not shown in Peaceful, at
  18 food or more, or while poisoned, withered or regenerating.
- Everything sits on the icons `HealthHud::layout` drew that frame
  (`BarPositions`), so food jitter and extra heart rows are followed. Mounted,
  the food bar is replaced by the mount's hearts and no food overlay is drawn.
- **Tooltips**: food items get the mod's row of hunger icons over saturation
  icons (outlined when a value differs from the default, `x12` past ten),
  without Shift unless `appleSkinTooltipsAlways` is off. **F3**: the line
  `hunger: 20, sat: 5, exh: 1.23/4`.
- Logic and tests: `appleskin.rs`; the game side `game/appleskin.rs`.

### Jade (`jade`, default off; `jadeHarvest` and `jadeDetails`, on)
A rewrite of the Java mod: a box at the top of the screen for the block or mob
under the crosshair.
- The block or mob's icon (the block item, or the mob's spawn egg), its name from
  the language files, and the owner on the last line (`Minecraft`, or a mod's
  name) in blue italics.
- Mobs: their health as hearts (up to ten, scaled for bigger pools).
- Blocks: crop growth, redstone power, repeater delay, comparator mode, note
  block note and instrument, composter, honey and cauldron levels, farmland
  moisture; `Unbreakable` for bedrock; which tool breaks it best and whether the
  held item can harvest it (`Items::correct_for_drops`).
- A mob's health is Jade's `HealthElement`: hearts, 4 pixels between rows when
  there are more than ten, and only above 40 half-hearts one heart and the number
  (`20/20` style, no fractions). A mob with armor gets Jade's armor row the same
  way. The box is Jade's default dark one (70% opacity), its text colours the
  default palette.
- A green bar under the box shows how far the block being dug has got.
- Logic and tests: `jade.rs`; the game side `game/jade.rs`.

### JEI (`jei`, default off)
A rewrite of the Java mod: on the inventory, container and creative screens, an
item list beside the menu and a recipe window.
- The list is every creative item, paged, searched by the box under it as JEI
  does: words in the name; `@mod`, `#tooltip`, `$tag` and `&id` prefixes; `-` to
  leave out, `|` for either, quotes for phrases.
- Click an item for how it is made, right-click for what uses it; R and U do the
  same over any item (the list, the window, the menu's own slots); Backspace goes
  back; Escape leaves the search box, then closes the window; the wheel turns
  pages, or steps through recipes in the window.
- The window looks like the mod's: category tabs above it (crafting, smelting,
  blasting, smoking, campfire, stonecutting, smithing), a title row with arrows
  to the next category, a page row, then as many recipes as fit, each in its
  frame. Shaped recipes keep their grid, shapeless ones fill it in order,
  ingredient tags cycle through their items each second, and items in the
  window can be clicked too.
- The `+` beside a recipe puts its ingredients in the inventory in creative
  mode (survival says it needs creative).
- The list has no slot boxes, a lit square under the pointer, page arrows, the
  search box and a wrench that opens the extras settings.
- Recipes come from the prepared data pack (`recipe_table.rs`, read once).
- Logic and tests: `jei.rs`; the game side `game/jei.rs`.

### Controlling (`controlling`, default off)
A port of Jaredlll08's Controlling (MIT) and its Searchables search, for the
Key Binds page: a wide search box (`name`, `category:` and `key:` terms, quotes
for phrases), footer buttons Toggle Free (every key no binding uses, with its
GLFW value), Sort (None, A->Z, Z->A, Key A->Z, Key Z->A), Show Unbound and Show
Conflicts (each toggles back to Show All), and Reset All that asks to
`Confirm?` first. A search or filter shows a flat list; otherwise the
categories. Logic and tests: `controlling.rs`; the page in `options_ui.rs`.

### Connected textures (a resource pack feature, always on)
Resource packs in the OptiFine `optifine/ctm` format (what Continuity reads)
connect: glass that loses its border against more glass, bookshelves, sandstone
walls... The methods `ctm`, `horizontal`, `vertical`, `horizontal+vertical`,
`vertical+horizontal`, `top`, `random`, `repeat` and `fixed` work, with
`matchBlocks` (block ids and property values) and `matchTiles` (sprites),
`connect=block|tile|state`, `innerSeams`, `faces`, `heights`, `orient`,
`symmetry`, `weights`, `linked` and `prioritize`. Not done: the overlay
methods, `ctm_compact`, `biomes` and `name` filters (those rules are logged and
skipped). The tiles are extra layers of the block sprite array, so a connected
face costs nothing more to draw. Logic and tests: `ctm.rs`; the mesher asks it
for each face's layer.

`python tools/connected_glass.py` makes a pack of connected glass (plain and
all stained colours) from the game's own textures into `resourcepacks/`; turn
on "Horizon Connected Glass" in Resource Packs.

Continuity's own default pack (its connected glass in every colour and the panes, bookshelves and the
sandstone family, with its artwork and rules) is in the Continuity jar: `python tools/continuity_pack.py JAR`
makes `resourcepacks/Continuity Default.zip` from it (the programmer art variants, which Continuity only
applies with that resource pack, are left out). Turn it on in Resource Packs instead of "Horizon Connected
Glass" (they cover the same glass).

## Shared rules

- All extras are hidden with F1 and drawn in the HUD layer: under screens, over
  the world, like vanilla's HUD. Extras in the top left corner give way to the
  debug screen (F3), which shows the FPS anyway.
- Corners stack: extras placed in the same corner stack downward (top corners)
  or upward (bottom corners) in the order minimap, FPS, keystrokes. The top
  right corner starts below the status effect icons.

## Perf budget

Off: a flag check per frame per extra (no allocations, no world access).
On: FPS and keystrokes are a few quads; the minimap stays under 0.3 ms of CPU
per frame (tile budget) plus a 64 KiB texture upload when its image changes.
