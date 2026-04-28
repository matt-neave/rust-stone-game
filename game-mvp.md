Big rock, player clicks it 10 times, it produces small rock. (A rock is like the health packs from SNKRX but gray). Small ones are the same size as the rocks in SNKRX. The big one is much bigger. Rock particles come out the rock as the player clicks the rock. On the 10th click, a rock falls and randomly falls somewhere on the right hand side of the rock.
When a player clicks a rock, the rock is thrown into the water. A rock will skim along the water. There is a 50% chance the rock will bounce, and a 50% chance it will fall into the water. For each bounce, the player gains a currency (+1 skims). Skims is the main currency.
When a rock falls into the water, it is lost.
When a rock bounces, we show a marker for the +1.
When the rock bounces, or falls, we need a ripple shader in the water to indicate impact against the water.
Use music and SFX from the SNKRX package.
Keep the style in the pixelated style of SNKRX. The LHS of the screen will be sandy beach. The RHS will be water. Both minimally styalised with the pixelation effect.
The large rock will be at the left most side of the screen.
The skims currency label at the top of the screen.
Small rocks fall and land on the sand.

---

## Progress (post-MVP)

The original MVP loop is in place. Everything below has been added on top.

### Economy / structures

- **Cave** — pre-existing on the sand. Hover to surface its dynamic panel of one-time structure unlocks (rows reveal as prereqs are met, owned rows render sold-out).
- **Foragers Hut** (`Hut`, 10 skims) — sells `Worker` (cookie-clicker style 1.15× compounding price). 2 free starter workers on purchase.
- **Miners Hut** (`HutMiner`, 10 skims) — sells `Miner` (1 worker → pickaxe-thrower, ~10s cycle, 3 base damage to the big rock) and `MinerDamage` (30 skims, +1 damage per throw, stacks linearly).
- **Skimmers Hut** (`HutSkimmer`, 10 skims) — sells `Skimmer` (1 worker → walks idle rocks to the water and skims them, 25% bounce chance) and `SkimUpgrade` (25 skims, +5% bounce chance for skimmers only — cursor stays unbuffed; caps at 95%).
- **Anglers Hut** (`HutFisher`, 10 skims) — sells `Fisherman` (1 worker → fishes stones from the sea, 7-13s per cast, 50% catch).
- **Combers Hut** (`HutBeachcomber`, 15 skims) — sells `Beachcomber` (1 worker → walks the sand digging up free stones).
- **Masons Hut** (`HutStonemason`, 15 skims) — sells `Stonemason` (1 worker → claims an idle rock, chisels it, marks it `Masoned`).
- **Pier** (`Pier`, 30 skims) — gated behind owning skimmers OR fishermen. Sells `Fish` (5 skims, bucket of 10 one-shot fish that rescue failing bounces) and `Port`.
- **Port** (`Port`, 50 skims) — gated behind pier. Sells `Boatman` from its own panel.

Each specialist hut comes with 2 starter workers and is positioned to keep the layout clear of the small-rock landing zone.

### Crew specialists

| Role | Source hut | Behaviour |
|---|---|---|
| Worker | Foragers | Idles around the hut; consumed by specialist conversions. |
| Miner | Miners | Walks to a throw spot, hurls a pickaxe at the big rock. Damage scales with `MinerDamage` upgrade. |
| Skimmer | Skimmers | Picks up an idle rock, walks to the shore, skims it. Bounce odds scale with `SkimUpgrade`. |
| Fisherman | Anglers | Sits at the shoreline casting; on a catch a rock arcs onto the beach behind them. |
| Beachcomber | Combers | Walks random sand spots, digs (~3s), spawns a free small rock that pops up beside them. |
| Stonemason | Masons | Claims the nearest non-`Masoned` `Idle` rock, walks to it, chisels for ~3s, applies the `Masoned` marker (lighter colour + 2 guaranteed bounce charges). |
| Boatman | Port | Sails out to nearest `Sunken` rock, claims it, picks up; carries up to 5 stones; sails home and flings them onto the sand using player-grade bounce odds (with `SkimUpgrade` bonus). |

### Sunken-rock ecosystem

- Without a port, sunk rocks despawn (original MVP behaviour).
- With a port, sinks instead transition the rock to `SmallRockPhase::Sunken { pos }` (hidden) — boatmen ferry these home and re-launch them, so no rock is lost permanently.
- Boatmen claim their target sunken rock at search time so multiple boatmen don't dog-pile the same one.

### Masoned rocks

- New `Masoned { remaining: u8 }` component (default 2 charges).
- Charges burn before any bounce dice roll: a sink that would have happened becomes a guaranteed bounce, and `remaining` decrements. Once exhausted, normal odds resume.
- Visual: rock material shader gained a `brightness` uniform; masoned rocks render at `1.45×` for a clearly lighter, sharpened look.

### UI / hover

- Two-stage hover model: panels open only when the cursor enters the building footprint, and persist while the cursor sits on the panel chrome. Chrome blocks building hover-through.
- Cave panel relayout every frame from `CavePanelGeo` so it grows/shrinks symmetrically as new structure unlocks become visible.
- Each new specialist hut, Port, and pier panel has its own `PanelKind` and dedicated detail panel (header + multi-line body, recoloured by affordability).
- HUD: skims count, worker count, FPS counter, mute toggle, dock/window/fullscreen mode buttons.

### Display modes

- `Windowed` (default, 1280×720 centered), `Fullscreen`, `Docked` (UI hidden, useful for streaming a thin strip).
- Custom upscale RTT pipeline; docked mode crops the strip via `Sprite::rect`.

### Audio

- SNKRX SFX + music. Mute defaults to ON. `update_music_volume` handles the toggle live.

### Rendering

- `RockLitMaterial` (custom Material2d) with WGSL fragment shader: per-pixel directional sphere-bulge lighting, rotation-aware so the highlight stays anchored to world-space top-right while a rock spins. `brightness` uniform drives the masoned look.
- Water gradient + foam strip at the shoreline; sand patches; rock shadows; sand imprints where a rock used to sit; ripple shader on water impacts.
