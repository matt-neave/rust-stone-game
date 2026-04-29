//! Wood pieces — the post-research-mission tree click loop.
//!
//! Once `ResearchMission.unlocked` is true the tree becomes
//! click-targetable. Mirroring the big-rock click counter, each click
//! within the tree's foliage radius increments a `TreeClicks` counter;
//! every `CLICKS_PER_WOOD` clicks the counter resets and one
//! `WoodPiece` arcs from the foliage to a random nearby sand spot.
//! Pieces in the `Idle` phase are themselves clickable; clicking one
//! either flings it back into the [`TreeStorage`] (when the player
//! has built one — the wood resource ticks up by 1 on arrival) or
//! kicks it to a fresh random ground spot.
//!
//! Falling and jumping arcs use `ZHeight` (same plumbing as small
//! rocks), so wood visibly leaves the ground while a paired shadow
//! tracks the landing spot. Each click also nudges a tiny per-frame
//! wiggle on the foliage's transform — quick visual confirmation that
//! the click registered without spawning a piece.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::assets::GameAssets;
use crate::core::colors;
use crate::core::common::{Layer, Pos, SyncSet, ZHeight};
use crate::core::constants::{
    CLICKS_PER_WOOD, TREE_STORAGE_X, TREE_STORAGE_Y, TREE_X, TREE_Y, WOOD_CLICK_R,
    WOOD_LAND_X_MAX, WOOD_LAND_X_MIN, WOOD_LAND_Y_MAX, WOOD_LAND_Y_MIN, Z_PARTICLE, Z_UI,
};
use crate::core::input::{run_autoclick, ClickEvent};
use crate::currency::Wood;
use crate::economy::{ResearchMission, TreeStorage};
use crate::effects::particles::SpawnParticleBurstEvent;
use crate::effects::SpawnFloatingTextEvent;
use crate::render::shapes::Shapes;
use crate::render::{CameraScroll, DisplayMode, DisplayScale, UiText, UI_LAYER};
use crate::rocks::shadow::spawn_rock_shadow;
use crate::world::bg::TreeFoliage;

/// Click radius around the tree's foliage centre.
const TREE_CLICK_R: f32 = 16.0;

/// Wiggle envelope tuning — kept subtle so the pixel-art tree never
/// looks like it's rocking back and forth. 1.5 px max horizontal
/// shake, ~5 Hz oscillation, fully decayed in 0.2 s.
const WIGGLE_AMPLITUDE: f32 = 1.5;
const WIGGLE_FREQUENCY: f32 = 30.0;
const WIGGLE_DURATION: f32 = 0.2;

/// Wood-piece silhouette size in spec px. Slightly elongated so the
/// 6×3 baked log mask reads as a little fallen branch.
const WOOD_SPRITE_SIZE: Vec2 = Vec2::new(6.0, 3.0);

#[derive(Component)]
pub struct WoodPiece {
    pub state: WoodPhase,
}

pub enum WoodPhase {
    Falling {
        from: Vec2,
        to: Vec2,
        time: f32,
        dur: f32,
    },
    Idle,
    Jumping {
        from: Vec2,
        to: Vec2,
        time: f32,
        dur: f32,
        into_storage: bool,
    },
}

/// Click counter for the tree — every `CLICKS_PER_WOOD` clicks
/// triggers one wood-piece spawn, mirroring the big-rock pattern.
#[derive(Resource, Default)]
pub struct TreeClicks {
    pub count: u32,
}

/// Hold-to-autoclick accumulator for the tree. Mirrors
/// `BigRockHoldState` — while LMB is held with the cursor over the
/// foliage, synthetic `ClickEvent`s fire every `AUTOCLICK_INTERVAL`.
#[derive(Resource, Default)]
pub struct TreeHoldState {
    pub accum: f32,
}

/// Per-frame foliage shake state. Reset to `time = 0` on every
/// registered tree click so chained clicks feel snappy.
#[derive(Component, Default)]
pub struct TreeWiggle {
    pub time: f32,
    pub active: bool,
}

/// Marker for the small "n/25" counter text that floats above the
/// foliage. World-space spec position; the global `sync_ui_text`
/// scroll subtraction keeps it pinned to the tree as the camera
/// pans.
#[derive(Component)]
pub struct TreeClickCounterText;

pub struct WoodPlugin;

impl Plugin for WoodPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TreeClicks>()
            .init_resource::<TreeHoldState>()
            .add_systems(
                Update,
                (
                    tree_autoclick,
                    tree_click_spawn,
                    wood_click_handler,
                    tick_wood,
                    update_tree_counter_text,
                    update_tree_counter_visibility,
                ),
            )
            // Wiggle runs *after* `sync_transforms` so the additive
            // shake on the foliage's translation isn't immediately
            // clobbered by the Pos-driven rewrite.
            .add_systems(PostUpdate, tick_tree_wiggle.after(SyncSet::Transforms))
            .add_systems(Startup, (attach_tree_wiggle, spawn_tree_counter_text));
    }
}

/// While LMB is held with the cursor over the tree's foliage, emit
/// synthetic `ClickEvent`s — gated on `mission.unlocked` so the tree
/// only autoclicks once the cinematic completes. Shares its
/// bookkeeping with the big rock via `run_autoclick`.
#[allow(clippy::too_many_arguments)]
fn tree_autoclick(
    time: Res<Time>,
    mouse: Res<bevy::input::ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    display_scale: Res<DisplayScale>,
    mode: Res<DisplayMode>,
    scroll: Res<CameraScroll>,
    mission: Res<ResearchMission>,
    mut hold: ResMut<TreeHoldState>,
    mut writer: MessageWriter<ClickEvent>,
) {
    run_autoclick(
        time.delta_secs(),
        &mouse,
        windows.single().ok(),
        display_scale.0,
        *mode,
        scroll.x,
        mission.unlocked,
        Vec2::new(TREE_X, TREE_Y - 9.0),
        TREE_CLICK_R,
        &mut hold.accum,
        &mut writer,
    );
}

fn random_ground_spot(rng: &mut impl Rng) -> Vec2 {
    Vec2::new(
        rng.gen_range(WOOD_LAND_X_MIN..WOOD_LAND_X_MAX),
        rng.gen_range(WOOD_LAND_Y_MIN..WOOD_LAND_Y_MAX),
    )
}

/// Attach a `TreeWiggle` to the foliage entity at startup. Runs once
/// in `Startup` after `bg::spawn_tree` (Bevy's default startup order
/// runs systems registered earlier first; both plugins use `Startup`,
/// and the foliage entity exists by the time this runs because Bevy
/// flushes commands between Startup stages — but to be safe we use a
/// query that simply skips when the entity isn't there yet).
fn attach_tree_wiggle(
    mut commands: Commands,
    q: Query<Entity, (With<TreeFoliage>, Without<TreeWiggle>)>,
) {
    for e in &q {
        commands.entity(e).insert(TreeWiggle::default());
    }
}

fn spawn_tree_counter_text(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn((
        TreeClickCounterText,
        UiText {
            spec_pos: Vec2::new(TREE_X, TREE_Y - 25.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new(format!("0/{}", CLICKS_PER_WOOD)),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG),
        Transform::default(),
        Anchor::CENTER,
        RenderLayers::layer(UI_LAYER),
        Visibility::Hidden,
    ));
}

#[allow(clippy::too_many_arguments)]
fn tree_click_spawn(
    mut clicks: MessageReader<ClickEvent>,
    mission: Res<ResearchMission>,
    mut tree_clicks: ResMut<TreeClicks>,
    mut wiggle_q: Query<&mut TreeWiggle>,
    mut commands: Commands,
    shapes: Res<Shapes>,
    mut sound: MessageWriter<PlaySoundEvent>,
    mut burst: MessageWriter<SpawnParticleBurstEvent>,
) {
    if !mission.unlocked {
        // Drain — other handlers also read events, but no point in
        // doing the per-click distance check when the feature is off.
        for _ in clicks.read() {}
        return;
    }
    let foliage_center = Vec2::new(TREE_X, TREE_Y - 9.0);
    let mut rng = rand::thread_rng();
    for ev in clicks.read() {
        if foliage_center.distance(ev.pos) > TREE_CLICK_R {
            continue;
        }

        // Shake the foliage and emit a tiny puff so the click reads
        // even on counter-only ticks.
        for mut w in &mut wiggle_q {
            w.time = 0.0;
            w.active = true;
        }
        burst.write(SpawnParticleBurstEvent {
            pos: ev.pos,
            color: colors::TREE_FOLIAGE_LIGHT,
            count: 3,
            angle_min: -std::f32::consts::PI,
            angle_max: 0.0,
            speed_min: 20.0,
            speed_max: 50.0,
            size_min: 1.0,
            size_max: 1.5,
            lifetime_min: 0.18,
            lifetime_max: 0.32,
            damping: 1.6,
            gravity: 60.0,
        });
        sound.write(PlaySoundEvent {
            kind: SoundKind::Click,
            pitch: 1.25,
            volume: 0.2,
        });

        tree_clicks.count = tree_clicks.count.saturating_add(1);
        if tree_clicks.count < CLICKS_PER_WOOD {
            continue;
        }
        // Counter trips — drain one wood-piece worth and spawn.
        tree_clicks.count -= CLICKS_PER_WOOD;

        // Random ground spot well clear of the tree — fans wood out
        // into a satisfying spread instead of piling at the trunk.
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let radius = rng.gen_range(30.0..60.0);
        let raw = foliage_center + Vec2::from_angle(angle) * radius;
        let to = Vec2::new(
            raw.x.clamp(WOOD_LAND_X_MIN, WOOD_LAND_X_MAX),
            raw.y.clamp(WOOD_LAND_Y_MIN, WOOD_LAND_Y_MAX),
        );

        // Random small in-flight rotation seed so individual pieces
        // don't all face the same way after settling. Final settle
        // rotation is also randomized in `tick_wood` on landing.
        let initial_rot = rng.gen_range(-0.5_f32..0.5_f32);
        let rock_e = commands
            .spawn((
                WoodPiece {
                    state: WoodPhase::Falling {
                        from: foliage_center,
                        to,
                        time: 0.0,
                        dur: 0.5,
                    },
                },
                Pos(foliage_center),
                ZHeight(0.0),
                Layer(Z_PARTICLE),
                Sprite {
                    image: shapes.log.clone(),
                    color: colors::TREE_TRUNK,
                    custom_size: Some(WOOD_SPRITE_SIZE),
                    ..default()
                },
                Transform::from_rotation(Quat::from_rotation_z(initial_rot)),
            ))
            .id();
        // Paired shadow that tracks the wood's ground position. The
        // `Shadow` system already reads `ZHeight` to fade and shrink,
        // but it filters on `With<SmallRock>` — so shadows on wood
        // simply stay at base alpha. Still helps land the arc.
        spawn_rock_shadow(&mut commands, &shapes, rock_e, foliage_center, WOOD_SPRITE_SIZE);
        sound.write(PlaySoundEvent {
            kind: SoundKind::SmallRockSpawn,
            pitch: 1.1,
            volume: 0.25,
        });
    }
}

fn wood_click_handler(
    mut clicks: MessageReader<ClickEvent>,
    storage: Res<TreeStorage>,
    mut q: Query<(&mut WoodPiece, &Pos)>,
) {
    let mut rng = rand::thread_rng();
    for ev in clicks.read() {
        // Find nearest Idle piece within the click radius.
        let mut best: Option<(f32, Mut<WoodPiece>, Vec2)> = None;
        for (piece, pos) in q.iter_mut() {
            if !matches!(piece.state, WoodPhase::Idle) {
                continue;
            }
            let d = pos.0.distance(ev.pos);
            if d > WOOD_CLICK_R {
                continue;
            }
            if best.as_ref().map_or(true, |(bd, _, _)| d < *bd) {
                best = Some((d, piece, pos.0));
            }
        }
        let Some((_, mut piece, from)) = best else { continue };
        if storage.owned {
            // Snappy arrival into the storage box.
            piece.state = WoodPhase::Jumping {
                from,
                to: Vec2::new(TREE_STORAGE_X, TREE_STORAGE_Y),
                time: 0.0,
                dur: 0.3,
                into_storage: true,
            };
        } else {
            let to = random_ground_spot(&mut rng);
            piece.state = WoodPhase::Jumping {
                from,
                to,
                time: 0.0,
                dur: 0.4,
                into_storage: false,
            };
        }
    }
}

fn tick_wood(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut WoodPiece, &mut Pos, &mut ZHeight, &mut Transform)>,
    mut wood: ResMut<Wood>,
    mut floating: MessageWriter<SpawnFloatingTextEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (entity, mut piece, mut pos, mut zh, mut tf) in &mut q {
        let next: Option<WoodPhase> = match &mut piece.state {
            WoodPhase::Falling { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                let dx = (to.x - from.x).abs().max(20.0);
                let apex = (dx * 0.4).clamp(8.0, 30.0);
                zh.0 = apex * 4.0 * prog * (1.0 - prog);
                // Half a rotation across the arc — reads as tumbling.
                tf.rotation = Quat::from_rotation_z(prog * std::f32::consts::PI);
                if *t >= *dur {
                    pos.0 = *to;
                    zh.0 = 0.0;
                    tf.rotation =
                        Quat::from_rotation_z(rng.gen_range(-0.5_f32..0.5_f32));
                    Some(WoodPhase::Idle)
                } else {
                    None
                }
            }
            WoodPhase::Idle => None,
            WoodPhase::Jumping { from, to, time: t, dur, into_storage } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                // Smaller apex for storage arrivals, fuller arc when
                // re-tossing to a new ground spot.
                let dx = (to.x - from.x).abs().max(20.0);
                let apex_base = if *into_storage { 0.25 } else { 0.4 };
                let apex = (dx * apex_base).clamp(if *into_storage { 6.0 } else { 8.0 }, 24.0);
                zh.0 = apex * 4.0 * prog * (1.0 - prog);
                tf.rotation = Quat::from_rotation_z(prog * std::f32::consts::PI);
                if *t >= *dur {
                    if *into_storage {
                        wood.total = wood.total.saturating_add(1);
                        floating.write(SpawnFloatingTextEvent::reward(
                            *to,
                            "+1",
                            colors::TREE_FOLIAGE_LIGHT,
                        ));
                        commands.entity(entity).despawn();
                        None
                    } else {
                        pos.0 = *to;
                        zh.0 = 0.0;
                        tf.rotation =
                            Quat::from_rotation_z(rng.gen_range(-0.5_f32..0.5_f32));
                        Some(WoodPhase::Idle)
                    }
                } else {
                    None
                }
            }
        };
        if let Some(s) = next {
            piece.state = s;
        }
    }
}

/// Per-frame foliage shake. Runs in `PostUpdate` after
/// `sync_transforms` (the system that writes `Transform.translation`
/// from `Pos`), so the shake offset layers cleanly on top of the
/// already-synced base position without fighting the canonical sync.
fn tick_tree_wiggle(
    time: Res<Time>,
    mut q: Query<(&mut TreeWiggle, &mut Transform), With<TreeFoliage>>,
) {
    let dt = time.delta_secs();
    for (mut w, mut tf) in &mut q {
        if !w.active {
            continue;
        }
        w.time += dt;
        if w.time >= WIGGLE_DURATION {
            w.active = false;
            // Base translation is already correct (just-synced from
            // Pos); nothing to undo.
            continue;
        }
        let envelope = 1.0 - (w.time / WIGGLE_DURATION);
        let shake = WIGGLE_AMPLITUDE * (w.time * WIGGLE_FREQUENCY).sin() * envelope;
        tf.translation.x += shake;
    }
}

fn update_tree_counter_text(
    clicks: Res<TreeClicks>,
    mut q: Query<&mut Text2d, With<TreeClickCounterText>>,
) {
    if !clicks.is_changed() {
        return;
    }
    for mut t in &mut q {
        t.0 = format!("{}/{}", clicks.count, CLICKS_PER_WOOD);
    }
}

fn update_tree_counter_visibility(
    mission: Res<ResearchMission>,
    mut q: Query<&mut Visibility, With<TreeClickCounterText>>,
) {
    if !mission.is_changed() {
        return;
    }
    let want = if mission.unlocked {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in &mut q {
        if *v != want {
            *v = want;
        }
    }
}
