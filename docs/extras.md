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
- A light source held by a player or mob (main or off hand), dropped as an item, or an entity on
  fire lights the blocks, entities and hand around it, as LambDynamicLights and OptiFine do. Not
  in vanilla; the idea is ported, not a mod's code (issue #814).
- An item glows like its block's default state (`getLightEmission`): torches 14, lanterns,
  glowstone, sea lanterns and jack o'lanterns 15, soul torches 10, a redstone torch 7; a lava bucket
  15. An entity on fire gives 15.
- Client-only, at render time: the light engine, meshes, saves and other players never see it.
  Each frame the game picks the 16 sources nearest the camera (within 96 blocks,
  `dynamic_light::select`); terrain.wgsl and entity.wgsl raise each vertex's block light to the
  source's level minus its distance in blocks when that is brighter, then light it through the
  lightmap as usual. The light doesn't stop at walls.
- Off: nothing is gathered and the shaders' loop runs zero times (the count is 0), so it costs
  nothing measurable (numbers in the pull request, #814).

### Zoom (`zoom`, default off)
- Hold the zoom key (default **Z**, unused by every vanilla 26.3 key mapping; C
  is `key.saveToolbarActivator`) to zoom. The key does nothing while zoom is off.
- It zooms only while playing: in chat, signs, books, text fields and every
  other screen the key is typed or handled as usual. The key is never taken
  from vanilla's mappings: one bound to the same key still acts (a key bound
  twice triggers both, as vanilla's do), and the Extras page marks the zoom key
  "Also bound" and names the mapping.
- Starts at the default level (4x, settable 2x to 10x); the scroll wheel changes
  it while the key is held (and does not move the hotbar then).
- **Smooth** (default on) goes through vanilla's own FOV easing: the zoom divides
  the field-of-view modifier target `Camera.tickFov` eases toward, the path the
  spyglass uses (`AbstractClientPlayer.getFieldOfViewModifier` returns 0.1 while
  scoping). The modifier keeps vanilla's 0.1 floor, so 10x is the deepest zoom.
  Off applies the zoom to the final FOV with no easing.
- While scoping with a spyglass the zoom does nothing: the spyglass view and its
  overlay stay exactly vanilla's.
- **Slower turning** (default on) divides mouse turning by the zoom factor.
- With zoom off (or the key up) the FOV target and mouse are vanilla's, bit for
  bit; the FOV harness covers this.

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
In the player inventory and chest-like containers (chests, barrels, ender
chests, shulker boxes, hoppers, dispensers), in singleplayer and on servers:
- Right-drag holding a stack: one item goes into each slot entered, at once
  (slots holding another item, or full, are skipped).
- Left-drag holding a stack: each slot entered that holds the same item is
  gathered into the hand, when it all fits. This takes over only while
  vanilla's drag covers one slot, so a left-drag across empty slots still
  splits the stack as vanilla does.
- Shift + left-drag: every slot entered is quick-moved (with a stack in hand,
  only the slots of its item).
- The wheel over a slot, with an empty hand (`mouseTweaksWheel`): down moves
  one item to the other side (container and inventory, or hotbar and main
  inventory: to a stack of it with room, else the first empty slot, as
  quick-move picks), up brings one of the same item back.

Every action is ordinary slot clicks (`ContainerClick` on a server), so a
vanilla server sees nothing unusual. With the extra off, the hooks return at
once and the screens click exactly as vanilla's (`tools/test_menu_click_parity.py`
and the container screen tests are unchanged).

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
A rewrite of the Java mod (public domain) for this client's own HUD. Sub-switches
(all on with the extra): `appleSkinRestored`, `appleSkinSaturation`,
`appleSkinExhaustion`, `appleSkinHealth`, `appleSkinTooltips`, `appleSkinDebug`.
- **Saturation**: a golden outline on each food icon, filled from the bottom in
  quarters as saturation covers the icon (10 icons, saturation / 2 each).
- **Exhaustion**: a pale strip under the food icons growing from the right end
  as exhaustion builds to 4 (the point where a unit of saturation or hunger goes).
- **Food in hand** (only when the player could eat it, `Player.canEat`): the
  hunger it would restore flashes on the food bar (vanilla's own food sprites at
  a pulsing alpha, `sin(tick * 0.1)`, at most 0.65), the saturation it would add
  flashes as outlines, and the hearts natural regeneration would then give flash
  on the health bar.
- The heart estimate runs vanilla's `FoodData.tick` forward on a stand-in world
  (health, difficulty, the natural regeneration rule), so it follows vanilla's
  rules exactly; it reruns only when the food state or health changes.
- Everything sits on the icons `HealthHud::layout` drew that frame
  (`BarPositions`), so food jitter, the regeneration wobble and extra heart rows
  are followed. Mounted, the food bar is replaced by the mount's hearts and
  nothing is drawn.
- **Tooltips**: food items list `Hunger: +N` and `Saturation: +S`, and what the
  bar cannot hold. **F3**: `hunger, saturation, exhaustion`.
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
- A green bar under the box shows how far the block being dug has got.
- Logic and tests: `jade.rs`; the game side `game/jade.rs`.

### JEI (`jei`, default off)
A rewrite of the Java mod: on the inventory, container and creative screens, an
item list beside the menu and a recipe window.
- The list is every creative item, paged, searched by the box under it with the
  creative screen's own search (item names, `#tag`).
- Click an item for how it is made, right-click for what uses it; R and U do the
  same over any item (the list, the window, the menu's own slots); Backspace goes
  back; Escape leaves the search box, then closes the window; the wheel turns
  pages, or steps through recipes in the window.
- The window shows crafting (shaped recipes keep their grid, shapeless ones fill
  it in order), smelting, blasting, smoking, campfire cooking, stonecutting,
  smithing and transmute recipes, with ingredient tags cycling through their
  items each second. Items in the window can be clicked too.
- Recipes come from the prepared data pack (`recipe_table.rs`, read once).
- Logic and tests: `jei.rs`; the game side `game/jei.rs`.

### Controlling (`controlling`, default off)
A rewrite of the Java mod for the Key Binds page: the search also matches the
key's name (`shift`, `R`) and the category, and a button next to the search box
cycles Show: All, Conflicts, Unbound, Changed.

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
