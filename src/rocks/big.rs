//! The big rock. Fixed-position clickable. Each click:
//!   * spits rock-dust particles
//!   * advances the click counter
//!   * plays a short click sound
//! Every `CLICKS_PER_SMALL_ROCK` clicks (10), the counter resets and a small
//! rock falls from the big rock to a random spot on the sand to its right.
//!
//! The "fall" is implemented as a tracked drop: the rock spawns at the big
//! rock's center with a small upward arc (visualized as a fall through the
//! air to its target sand spot), and on landing emits a chunky thud.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::*;
use crate::core::input::{cursor_to_spec, ClickEvent};
use crate::effects::particles::SpawnParticleBurstEvent;
use crate::render::{DisplayMode, DisplayScale, RockLitMaterial, RockLitParams, RockQuad, UiText, UI_LAYER};
use crate::render::shapes::Shapes;
use crate::rocks::shadow::spawn_static_shadow;
use crate::rocks::small::{SpawnSmallRockEvent, SMALL_ROCK_FALL_DURATION_FAST};

/// Hold-to-autoclick rate — 4 clicks per second. Synthetic clicks fire on
/// a fixed 0.25 s cadence after the initial press, so a player can
/// stockpile small rocks by just holding LMB on the boulder.
pub const AUTOCLICK_INTERVAL: f32 = 0.25;

#[derive(Component, Default)]
pub struct BigRock {
    pub clicks: u32,
}

#[derive(Component)]
pub struct BigRockHighlight;

/// Optional ECS-side feedback: a tiny pulse on the big rock's render scale
/// when clicked. Driven by a per-entity timer so multiple clicks chain.
#[derive(Component, Default)]
pub struct BigRockPulse {
    pub time: f32,
    pub amount: f32,
}

#[derive(Component)]
pub struct ClickCounterText;

/// Public hit-event consumed by the big rock — decoupled from raw mouse
/// clicks so non-player sources (miner pickaxes) can drive damage with
/// their own intensity. `damage` accumulates into the click counter and
/// can spawn multiple small rocks if it overshoots the threshold.
#[derive(Message)]
pub struct RockHitEvent {
    pub pos: Vec2,
    pub damage: u32,
}

/// Tracks how long LMB has been held over the big rock. Resets when the
/// button is released or the cursor leaves the rock's hitbox. Drives the
/// 2-cps autoclick.
#[derive(Resource, Default)]
pub struct BigRockHoldState {
    accum: f32,
}

pub struct BigRockPlugin;

impl Plugin for BigRockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BigRockHoldState>()
            .add_message::<RockHitEvent>()
            .add_systems(Startup, spawn_big_rock)
            // Autoclick before handle_clicks so any synthetic ClickEvents
            // it emits this frame are picked up immediately. handle_clicks
            // translates ClickEvents on the rock into RockHitEvents, then
            // apply_rock_hits is the single place that mutates the rock.
            .add_systems(
                Update,
                (
                    autoclick,
                    handle_clicks,
                    apply_rock_hits,
                    tick_pulse,
                    update_counter_text,
                )
                    .chain(),
            );
    }
}

fn spawn_big_rock(
    mut commands: Commands,
    shapes: Res<Shapes>,
    assets: Res<crate::core::assets::GameAssets>,
    rock_quad: Res<RockQuad>,
    mut materials: ResMut<Assets<RockLitMaterial>>,
) {
    // Static drop shadow under the boulder. Offset to the bottom-left
    // so it reads as cast by the top-right light source — half the
    // ellipse hides under the silhouette, the rest pokes out on the
    // sand to the lower-left.
    spawn_static_shadow(
        &mut commands,
        &shapes,
        Vec2::new(BIG_ROCK_X - 5.0, BIG_ROCK_Y + BIG_ROCK_H * 0.5 + 2.0),
        Vec2::new(BIG_ROCK_W * 0.82, 10.0),
        0.45,
    );

    // Body — boulder silhouette rendered through the rock shader so
    // its directional bands are computed live and stay anchored to
    // the world top-right light source even if the rock is ever
    // rotated.
    let material = materials.add(RockLitMaterial {
        silhouette: shapes.big_rock.clone(),
        params: RockLitParams::default(),
    });
    commands.spawn((
        BigRock::default(),
        BigRockPulse::default(),
        Pos(Vec2::new(BIG_ROCK_X, BIG_ROCK_Y)),
        Layer(Z_BIGROCK),
        Mesh2d(rock_quad.0.clone()),
        MeshMaterial2d(material),
        Transform::from_scale(Vec3::new(BIG_ROCK_W, BIG_ROCK_H, 1.0)),
    ));

    // Lighter highlight specks scattered over the upper-right of the rock
    // (a "lit from upper-right" feel). Offsets are local to the rock's
    // bounding-box center; sized 2-3 px each.
    let speck_offsets: [(f32, f32, f32); 6] = [
        (8.0, -14.0, 3.0),
        (15.0, -6.0, 2.0),
        (-2.0, -10.0, 2.0),
        (-12.0, 4.0, 2.0),
        (4.0, 6.0, 2.0),
        (18.0, 0.0, 2.0),
    ];
    for (dx, dy, size) in speck_offsets {
        commands.spawn((
            BigRockHighlight,
            Pos(Vec2::new(BIG_ROCK_X + dx, BIG_ROCK_Y + dy)),
            Layer(Z_BIGROCK + 0.1),
            Sprite::from_color(colors::ROCK_LIGHT, Vec2::splat(size)),
            Transform::default(),
        ));
    }
    // A few darker specks on the lower-left — implied shadow side.
    let shadow_offsets: [(f32, f32, f32); 3] = [
        (-14.0, 12.0, 2.0),
        (-6.0, 16.0, 2.0),
        (4.0, 18.0, 2.0),
    ];
    for (dx, dy, size) in shadow_offsets {
        commands.spawn((
            BigRockHighlight,
            Pos(Vec2::new(BIG_ROCK_X + dx, BIG_ROCK_Y + dy)),
            Layer(Z_BIGROCK + 0.1),
            Sprite::from_color(colors::ROCK_DARK, Vec2::splat(size)),
            Transform::default(),
        ));
    }

    // Click-progress text just below the rock.
    commands.spawn((
        ClickCounterText,
        UiText {
            spec_pos: Vec2::new(BIG_ROCK_X, BIG_ROCK_Y + BIG_ROCK_H * 0.5 + 6.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("0/10"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
    ));
}

/// While LMB is held with the cursor over the big rock, emit synthetic
/// `ClickEvent`s every `AUTOCLICK_INTERVAL` seconds. The initial press is
/// handled by `input::emit_clicks`; this system only fills in the held-down
/// repeat clicks. Skipped on the just-pressed frame so the press event
/// doesn't double-fire.
fn autoclick(
    time: Res<Time>,
    mouse: Res<bevy::input::ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    display_scale: Res<DisplayScale>,
    mode: Res<DisplayMode>,
    scroll: Res<crate::render::CameraScroll>,
    rock: Query<&Pos, With<BigRock>>,
    mut hold: ResMut<BigRockHoldState>,
    mut writer: MessageWriter<ClickEvent>,
) {
    // Docked mode is non-interactive — no autoclick.
    if *mode == DisplayMode::Docked {
        hold.accum = 0.0;
        return;
    }
    // Released — reset.
    if !mouse.pressed(MouseButton::Left) {
        hold.accum = 0.0;
        return;
    }
    // The press itself is the responsibility of input::emit_clicks. Just
    // start counting toward the next autoclick.
    if mouse.just_pressed(MouseButton::Left) {
        hold.accum = 0.0;
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(spec) = cursor_to_spec(window, display_scale.0, scroll.x) else {
        hold.accum = 0.0;
        return;
    };
    let Ok(rock_pos) = rock.single() else { return };
    if rock_pos.0.distance(spec) > BIG_ROCK_CLICK_R {
        // Cursor wandered off the rock — pause the timer so a later return
        // to the rock doesn't fire an immediate click.
        hold.accum = 0.0;
        return;
    }
    hold.accum += time.delta_secs();
    while hold.accum >= AUTOCLICK_INTERVAL {
        hold.accum -= AUTOCLICK_INTERVAL;
        writer.write(ClickEvent { pos: spec });
    }
}

fn handle_clicks(
    mut events: MessageReader<ClickEvent>,
    rock_q: Query<&Pos, With<BigRock>>,
    mut hits: MessageWriter<RockHitEvent>,
) {
    let Ok(pos) = rock_q.single() else { return };
    for ev in events.read() {
        // Hit test — circular hitbox around the rock center.
        if pos.0.distance(ev.pos) > BIG_ROCK_CLICK_R {
            continue;
        }
        hits.write(RockHitEvent {
            pos: ev.pos,
            damage: 1,
        });
    }
}

fn apply_rock_hits(
    mut events: MessageReader<RockHitEvent>,
    mut rock_q: Query<(&mut BigRock, &mut BigRockPulse, &Pos)>,
    mut burst: MessageWriter<SpawnParticleBurstEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
    mut spawn: MessageWriter<SpawnSmallRockEvent>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let Ok((mut rock, mut pulse, pos)) = rock_q.single_mut() else {
            continue;
        };

        rock.clicks = rock.clicks.saturating_add(ev.damage);

        // Visual + audio feedback for the hit itself.
        pulse.time = 0.0;
        pulse.amount = 0.18 + 0.05 * (ev.damage as f32 - 1.0).max(0.0);

        // Dust particles flying away from the impact point. Cone biased
        // toward the impact direction so it feels like material chipped
        // *away* from the source.
        let click_dir = (ev.pos - pos.0).normalize_or_zero();
        let base_angle = if click_dir == Vec2::ZERO {
            0.0
        } else {
            click_dir.y.atan2(click_dir.x)
        };
        // Heavier hits eject more dust.
        let dust_count = ROCK_DUST_PER_CLICK + (ev.damage.saturating_sub(1)) * 4;
        burst.write(SpawnParticleBurstEvent {
            pos: ev.pos,
            color: colors::ROCK_DARK,
            count: dust_count,
            angle_min: base_angle - 1.0,
            angle_max: base_angle + 1.0,
            speed_min: 50.0,
            speed_max: 130.0,
            size_min: 1.5,
            size_max: 2.5,
            lifetime_min: 0.25,
            lifetime_max: 0.55,
            damping: 1.4,
            // A whisper of gravity so dust settles toward the sand.
            gravity: 90.0,
        });
        burst.write(SpawnParticleBurstEvent {
            pos: ev.pos,
            color: colors::ROCK_LIGHT,
            count: 3 + ev.damage.saturating_sub(1),
            angle_min: base_angle - 0.6,
            angle_max: base_angle + 0.6,
            speed_min: 30.0,
            speed_max: 90.0,
            size_min: 1.0,
            size_max: 1.5,
            lifetime_min: 0.2,
            lifetime_max: 0.4,
            damping: 1.6,
            gravity: 60.0,
        });

        sound.write(PlaySoundEvent {
            kind: SoundKind::Click,
            pitch: rng.gen_range(0.95..1.10) * if ev.damage > 1 { 0.9 } else { 1.0 },
            volume: 0.35 + 0.05 * (ev.damage as f32 - 1.0).max(0.0).min(4.0),
        });

        // Drain the counter — overshoot from heavy hits spawns multiple
        // small rocks rather than rounding down to one.
        while rock.clicks >= CLICKS_PER_SMALL_ROCK {
            rock.clicks -= CLICKS_PER_SMALL_ROCK;
            let target = Vec2::new(
                rng.gen_range(SAND_LAND_X_MIN..SAND_LAND_X_MAX),
                rng.gen_range(SAND_LAND_Y_MIN..SAND_LAND_Y_MAX),
            );
            spawn.write(SpawnSmallRockEvent {
                from: pos.0,
                to: target,
                duration: SMALL_ROCK_FALL_DURATION_FAST,
            });
            sound.write(PlaySoundEvent {
                kind: SoundKind::SmallRockSpawn,
                pitch: 1.0,
                volume: 0.5,
            });
        }
    }
}

fn tick_pulse(
    time: Res<Time>,
    mut q: Query<(&mut BigRockPulse, &mut Transform), With<BigRock>>,
) {
    let dt = time.delta_secs();
    for (mut pulse, mut tf) in &mut q {
        pulse.time += dt;
        // 0.16 s pulse: scale starts at 1 + amount, eases back to 1.
        // The rock mesh is unit-sized, so the world scale always
        // multiplies the base BIG_ROCK_W/H — we can't rely on the
        // mesh "containing" its native size like the old sprite did.
        let dur: f32 = 0.16;
        let t = (pulse.time / dur).clamp(0.0, 1.0);
        let pulse_factor = 1.0 + pulse.amount * (1.0 - t);
        tf.scale.x = BIG_ROCK_W * pulse_factor;
        tf.scale.y = BIG_ROCK_H * pulse_factor;
    }
}

fn update_counter_text(
    rock_q: Query<&BigRock, Changed<BigRock>>,
    mut text_q: Query<&mut Text2d, With<ClickCounterText>>,
) {
    for rock in &rock_q {
        for mut t in &mut text_q {
            t.0 = format!("{}/{}", rock.clicks, CLICKS_PER_SMALL_ROCK);
        }
    }
}
