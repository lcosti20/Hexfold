//! Ambient sound: wind outside, a low hum inside, and footsteps while walking.
//!
//! Natively the three loops play through bevy_audio. On the web they play through Web Audio
//! directly ([`web::WebLoops`]): bevy_audio's web output (cpal) refills two ~46 ms buffers from
//! the main thread, so a long frame makes them overlap (doubled, choppy sound in Firefox), and
//! nothing resumes its `AudioContext` after the first click (silence in Edge and Chrome).

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::portal::PlayerSpace;

use super::player::Player;
use super::{AppState, Ending};

/// The loops, indexed by [`AmbientLoop`]: the outdoor wind, the interior hum, and the footsteps
/// (audible only while walking).
const LOOPS: [&str; 3] = ["sounds/wind_loop.wav", "sounds/interior_loop.wav", "sounds/footsteps.wav"];
const WIND: usize = 0;
const INTERIOR: usize = 1;
const FOOTSTEPS: usize = 2;

/// One of the [`LOOPS`] (native builds).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
pub struct AmbientLoop(usize);

/// Starts the three loops, silent; [`update_ambience`] sets their volumes.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_ambience(mut commands: Commands, assets: Res<AssetServer>) {
    use bevy::audio::{PlaybackMode, Volume};
    for (i, path) in LOOPS.iter().enumerate() {
        commands.spawn((
            Name::new(*path),
            AmbientLoop(i),
            AudioPlayer::new(assets.load(*path)),
            PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(0.0), ..default() },
        ));
    }
}

/// Starts the three loops, silent; [`update_ambience`] sets their volumes.
#[cfg(target_arch = "wasm32")]
pub fn spawn_ambience(mut commands: Commands) {
    match web::WebLoops::new(&LOOPS.map(|p| format!("assets/{p}"))) {
        Some(loops) => commands.insert_resource(loops),
        None => warn!("Web Audio unavailable: no sound"),
    }
}

/// Crossfades wind and hum as the player goes in and out (quieter wind after the hex breaks),
/// plays footsteps while the player walks on the ground, and silences everything off the
/// `Playing` screen.
pub fn update_ambience(
    time: Res<Time>,
    state: Res<State<AppState>>,
    space: Res<PlayerSpace>,
    ending: Res<Ending>,
    player: Query<(&LinearVelocity, &Player)>,
    #[cfg(not(target_arch = "wasm32"))] mut sinks: Query<(&AmbientLoop, &mut AudioSink)>,
    #[cfg(target_arch = "wasm32")] web: Option<Res<web::WebLoops>>,
    mut inside: Local<f32>,
) {
    let playing = *state.get() == AppState::Playing;
    // Crossfade between outside and inside over about a second.
    let want = if space.0.is_some_and(|s| s >= 1) { 1.0 } else { 0.0 };
    *inside += (want - *inside) * (time.delta_secs() * 2.0).min(1.0);
    let wind_level = if ending.broken { 0.25 } else { 1.0 };
    let walking = player.single().is_ok_and(|(v, p)| p.grounded && Vec2::new(v.x, v.z).length() > 0.5 * p.applied_scale);
    let mut volume = [0.0; 3];
    if playing {
        volume[WIND] = 0.5 * wind_level * (1.0 - *inside * 0.9);
        volume[INTERIOR] = 0.45 * *inside;
        volume[FOOTSTEPS] = if walking { 0.6 } else { 0.0 };
    }

    #[cfg(not(target_arch = "wasm32"))]
    for (l, mut sink) in &mut sinks {
        sink.set_volume(bevy::audio::Volume::Linear(volume[l.0]));
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(web) = web {
        web.set_volumes(&volume);
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use bevy::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{AudioBuffer, AudioContext, AudioContextState, GainNode, Response};

    /// Looping Web Audio sources, one gain node each. The browser loops them on its audio thread,
    /// so a slow frame can't make them stutter.
    #[derive(Resource)]
    pub struct WebLoops {
        ctx: AudioContext,
        gains: Vec<GainNode>,
    }

    // The web build is single-threaded (no `atomics`), so these JS handles never leave the thread.
    unsafe impl Send for WebLoops {}
    unsafe impl Sync for WebLoops {}

    impl WebLoops {
        /// Fetches, decodes and starts each loop at zero volume. Browsers create the context
        /// suspended until the page is clicked or a key is pressed, so it resumes on the first one.
        pub fn new(urls: &[String]) -> Option<Self> {
            let ctx = AudioContext::new().ok()?;
            let mut gains = Vec::new();
            for url in urls {
                let gain = ctx.create_gain().ok()?;
                gain.gain().set_value(0.0);
                gain.connect_with_audio_node(&ctx.destination()).ok()?;
                let (ctx, gain2, url) = (ctx.clone(), gain.clone(), url.clone());
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = start_loop(&ctx, &gain2, &url).await {
                        warn!("could not play {url}: {e:?}");
                    }
                });
                gains.push(gain);
            }
            resume_on_input(&ctx);
            Some(Self { ctx, gains })
        }

        /// Moves each loop towards its volume (a few ms of smoothing, so a step doesn't click).
        pub fn set_volumes(&self, volume: &[f32]) {
            let now = self.ctx.current_time();
            for (gain, &v) in self.gains.iter().zip(volume) {
                let param = gain.gain();
                if (param.value() - v).abs() > 1e-4 {
                    let _ = param.set_target_at_time(v, now, 0.015);
                }
            }
        }
    }

    async fn start_loop(ctx: &AudioContext, gain: &GainNode, url: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let response: Response = JsFuture::from(window.fetch_with_str(url)).await?.dyn_into()?;
        if !response.ok() {
            return Err(format!("HTTP {}", response.status()).into());
        }
        let bytes: js_sys::ArrayBuffer = JsFuture::from(response.array_buffer()?).await?.dyn_into()?;
        let buffer: AudioBuffer = JsFuture::from(ctx.decode_audio_data(&bytes)?).await?.dyn_into()?;
        let source = ctx.create_buffer_source()?;
        source.set_buffer(Some(&buffer));
        source.set_loop(true);
        source.connect_with_audio_node(gain)?;
        source.start()?;
        Ok(())
    }

    /// Resumes the context from inside the page's input handlers, where browsers allow it.
    fn resume_on_input(ctx: &AudioContext) {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
        let ctx = ctx.clone();
        let resume = Closure::<dyn FnMut()>::new(move || {
            if ctx.state() == AudioContextState::Suspended {
                let _ = ctx.resume();
            }
        });
        for event in ["pointerdown", "keydown", "touchend"] {
            let _ = document.add_event_listener_with_callback(event, resume.as_ref().unchecked_ref());
        }
        // Lives as long as the page.
        resume.forget();
    }
}
