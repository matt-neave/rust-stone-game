//! Sound + music. SFX picked from the SNKRX sound bank for a consistent
//! tonal palette. One-shot SFX are emitted via `PlaySoundEvent`; a single
//! music track loops at startup. The `Muted` resource gates everything —
//! music drops to zero volume while muted and one-shot SFX events are
//! drained without spawning sinks.

use bevy::audio::{AudioPlayer, AudioSink, AudioSource, PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, Debug)]
pub enum SoundKind {
    /// Big-rock click (every click).
    Click,
    /// 10th-click reward — small rock pops out.
    SmallRockSpawn,
    /// Rock lands on sand.
    Land,
    /// Skim bounce on water.
    SkimBounce,
    /// Rock sinks into water.
    Splash,
    /// Skim awarded — quick coin/bonus blip.
    Reward,
}

#[derive(Resource, Default)]
pub struct SoundLib {
    pub click: Vec<Handle<AudioSource>>,
    pub small_rock_spawn: Handle<AudioSource>,
    pub land: Handle<AudioSource>,
    pub skim_bounce: Vec<Handle<AudioSource>>,
    pub splash: Handle<AudioSource>,
    pub reward: Handle<AudioSource>,
    pub music: Handle<AudioSource>,
}

#[derive(Component)]
pub struct MusicPlayer;

#[derive(Message)]
pub struct PlaySoundEvent {
    pub kind: SoundKind,
    pub pitch: f32,
    pub volume: f32,
}

/// Global mute toggle. Owns by the audio module; flipped by the
/// mute button in `render::dock` and read by every audio system.
/// Defaults to **muted** so the game opens silently — players can
/// un-mute when they're ready.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Muted(pub bool);

impl Default for Muted {
    fn default() -> Self {
        Self(true)
    }
}

/// Music volume when the game isn't muted. Single source of truth so
/// the `update_music_volume` system can restore it on un-mute.
const MUSIC_VOLUME: f32 = 0.22;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundLib>()
            .init_resource::<Muted>()
            .add_message::<PlaySoundEvent>()
            .add_systems(PreStartup, load_sounds)
            // Music spawn is deferred to the first un-mute click rather
            // than firing in `Startup`. Browsers block audio until a
            // user gesture, and a sink created while the AudioContext
            // is still suspended can stay silent forever even after the
            // context unlocks. Waiting for the un-mute click guarantees
            // the gesture has already happened.
            .add_systems(Update, (play_sounds, ensure_music_started, update_music_volume));
    }
}

fn load_sounds(mut lib: ResMut<SoundLib>, server: Res<AssetServer>) {
    lib.click = vec![
        server.load("sounds/Click.ogg"),
        server.load("sounds/Pop sounds 10.ogg"),
        server.load("sounds/Concrete 6.ogg"),
    ];
    lib.small_rock_spawn = server.load("sounds/Unlock 3.ogg");
    lib.land = server.load("sounds/Wood Heavy 5.ogg");
    lib.skim_bounce = vec![
        server.load("sounds/Switch.ogg"),
        server.load("sounds/Spark 1.ogg"),
        server.load("sounds/Buff 13.ogg"),
    ];
    lib.splash = server.load("sounds/Body Fall 2.ogg");
    lib.reward = server.load("sounds/Coins 7.ogg");
    // "Cascade" — flowing, water-themed mid-album track. Good middle
    // ground between the ambient Glaciers and the more upbeat Pathfinder.
    lib.music = server.load("sounds/Kubbi - Ember - 04 Cascade.ogg");
}

fn play_sounds(
    mut commands: Commands,
    lib: Res<SoundLib>,
    muted: Res<Muted>,
    mut events: MessageReader<PlaySoundEvent>,
) {
    if muted.0 {
        // Drain pending events without spawning audio sinks so the
        // backlog doesn't burst out the moment the player un-mutes.
        for _ in events.read() {}
        return;
    }
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let handle = match ev.kind {
            SoundKind::Click => pick(&lib.click, &mut rng),
            SoundKind::SmallRockSpawn => Some(lib.small_rock_spawn.clone()),
            SoundKind::Land => Some(lib.land.clone()),
            SoundKind::SkimBounce => pick(&lib.skim_bounce, &mut rng),
            SoundKind::Splash => Some(lib.splash.clone()),
            SoundKind::Reward => Some(lib.reward.clone()),
        };
        let Some(handle) = handle else { continue };
        let pitch_jitter: f32 = rng.gen_range(0.95..1.05);
        let speed = ev.pitch * pitch_jitter;
        commands.spawn((
            AudioPlayer::<AudioSource>(handle),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(ev.volume.max(0.0)))
                .with_speed(speed.max(0.05)),
        ));
    }
}

fn pick<T: Clone>(slice: &[T], rng: &mut rand::rngs::ThreadRng) -> Option<T> {
    if slice.is_empty() {
        None
    } else {
        Some(slice[rng.gen_range(0..slice.len())].clone())
    }
}

/// Spawn the looping music sink the first time the player unmutes.
/// This deferral is what lets the game work on wasm: browsers won't
/// resume an `AudioContext` until a user gesture, and a sink created
/// before that gesture can stay silent forever. Pinning the spawn to
/// the unmute click guarantees the gesture has already happened.
fn ensure_music_started(
    mut commands: Commands,
    lib: Res<SoundLib>,
    muted: Res<Muted>,
    existing: Query<(), With<MusicPlayer>>,
) {
    if muted.0 || !existing.is_empty() {
        return;
    }
    if lib.music == Handle::<AudioSource>::default() {
        return;
    }
    commands.spawn((
        MusicPlayer,
        AudioPlayer::<AudioSource>(lib.music.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(MUSIC_VOLUME),
            ..PlaybackSettings::LOOP
        },
    ));
}

/// React to changes in `Muted` by silencing or restoring the looping
/// music track's sink. Runs only when the resource changes, so the
/// per-frame cost in normal play is one resource-changed check.
fn update_music_volume(
    muted: Res<Muted>,
    mut sinks: Query<&mut AudioSink, With<MusicPlayer>>,
) {
    if !muted.is_changed() {
        return;
    }
    let target = if muted.0 { 0.0 } else { MUSIC_VOLUME };
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(target));
    }
}
