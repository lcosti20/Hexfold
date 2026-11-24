//! A scripted driver for automated checks: `HEXFOLD_SCRIPT="wait:60;tp:0,2000,100,-90,0,0;shot:a;quit"`.
//!
//! Steps (separated by `;`):
//! * `wait:N`            do nothing for N frames
//! * `tp:x,y,z,yaw,pitch,space`  teleport the player (Unreal cm and degrees; space -1 = none)
//! * `walk:dx,dy,dz,frames`  push the player with a velocity (Unreal axes, m/s) for some frames
//! * `fwd:speed,frames`  walk the way the player faces (m/s), through portals too
//! * `turn:dyaw,dpitch,frames`  turn the view (degrees per frame) while the next steps run
//! * `key:E` / `key:Space`  tap a key: pressed for one frame (E grabs or drops)
//! * `click`             tap the left mouse button (throws what is held)
//! * `runetp:i,x,y,z`    teleport rune `i` (Unreal cm)
//! * `runevel:i,vx,vy,vz`  set rune `i`'s velocity (Unreal axes, m/s)
//! * `sunrise:f`         jump the ending to progress `f` (0 = night, 1 = dawn)
//! * `pos`               log the player and every rune (position, scale, mirrored)
//! * `shot:name`         save `shots/name.png`
//! * `burst:name,N`      save `shots/name_0000.png` ... for the next N frames, while the next steps run
//! * `quit`              exit
//!
//! Without the variable this does nothing.
//!
//! `HEXFOLD_TRACE` prints a line per frame (frame time, the player's position and velocity, portal
//! crossings, view count) and every contact on the player that isn't a level floor: what a bump
//! or a flicker looks like in numbers.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::portal::{PlayerSpace, PortalPawn, PortalTraveller};
use crate::ue;

/// One step of a script (see the module docs for the syntax).
#[derive(Debug, Clone)]
enum Step {
    /// Do nothing for N frames.
    Wait(u32),
    /// Teleport the player (Unreal cm and degrees; space -1 = none).
    Teleport { loc: [f32; 3], yaw: f32, pitch: f32, space: i32 },
    /// Set the player's horizontal velocity (Bevy axes, m/s) for some frames.
    Walk { v: Vec3, frames: u32 },
    /// Walk the way the player faces (m/s) for some frames, like holding W.
    Forward(f32, u32),
    /// Save a screenshot.
    Shot(String),
    /// Log the player and the runes.
    Pos,
    /// Set a rune's velocity (Bevy axes, m/s).
    RuneVel(u8, Vec3),
    /// Teleport a rune (Unreal cm).
    RuneTp(u8, [f32; 3]),
    /// Tap a key (released the next frame).
    Key(KeyCode),
    /// Tap the left mouse button (released the next frame).
    Click,
    /// Jump the ending to this progress (0 = night, 1 = dawn).
    Sunrise(f32),
    /// Turn the view by (yaw, pitch) degrees per frame for some frames, while later steps run.
    Turn(f32, f32, u32),
    /// Save a screenshot every frame for N frames (`shots/name_0000.png` ...), while later steps run.
    Burst(String, u32),
    /// Exit the game.
    Quit,
}

/// A running script: its steps and the steps still in progress (each with frames left).
#[derive(Resource, Default)]
pub struct DebugScript {
    steps: Vec<Step>,
    /// Index of the next step to start.
    next: usize,
    /// Frames left of a `wait` (or the pause after a `shot`).
    wait: u32,
    walk: Option<(Vec3, u32)>,
    forward: Option<(f32, u32)>,
    /// Turn per frame (degrees) and frames left; runs alongside later steps.
    turn: Option<(f32, f32, u32)>,
    /// Name, next frame number and total frames; runs alongside later steps.
    burst: Option<(String, u32, u32)>,
    /// Keys pressed last frame, released this frame (a scripted press is a one-frame tap).
    pressed_keys: Vec<KeyCode>,
    /// The left mouse button was pressed last frame.
    pressed_click: bool,
}

/// A debug setting: the environment variable `name`, or on the web the page's URL parameter of
/// the same name (`index.html?HEXFOLD_SCRIPT=wait:300;pos&HEXFOLD_TRACE=1`).
pub fn var(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    return std::env::var(name).ok();
    #[cfg(target_arch = "wasm32")]
    {
        let search = web_sys::window()?.location().search().ok()?;
        web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(name)
    }
}

/// Prints a debug line: to stderr, or on the web to the browser console.
pub fn log(line: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{line}");
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&line.into());
}

/// Parses `HEXFOLD_SCRIPT`, or returns `None` if it isn't set. Unknown steps are skipped with a
/// warning.
pub fn load_script() -> Option<DebugScript> {
    let text = var("HEXFOLD_SCRIPT")?;
    let mut steps = Vec::new();
    for raw in text.split(';') {
        let raw = raw.trim();
        let (cmd, arg) = raw.split_once(':').unwrap_or((raw, ""));
        let nums: Vec<f32> = arg.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        match cmd {
            "wait" => steps.push(Step::Wait(nums.first().copied().unwrap_or(1.0) as u32)),
            "tp" if nums.len() >= 3 => steps.push(Step::Teleport {
                loc: [nums[0], nums[1], nums[2]],
                yaw: nums.get(3).copied().unwrap_or(0.0),
                pitch: nums.get(4).copied().unwrap_or(0.0),
                space: nums.get(5).copied().unwrap_or(-1.0) as i32,
            }),
            "walk" if nums.len() >= 4 => steps.push(Step::Walk { v: Vec3::new(nums[0], nums[2], nums[1]), frames: nums[3] as u32 }),
            "fwd" if nums.len() >= 2 => steps.push(Step::Forward(nums[0], nums[1] as u32)),
            "shot" => steps.push(Step::Shot(arg.to_string())),
            "pos" => steps.push(Step::Pos),
            "key" => match arg { "E" => steps.push(Step::Key(KeyCode::KeyE)), "Space" => steps.push(Step::Key(KeyCode::Space)), _ => {} },
            "click" => steps.push(Step::Click),
            "runetp" if nums.len() >= 4 => steps.push(Step::RuneTp(nums[0] as u8, [nums[1], nums[2], nums[3]])),
            "runevel" if nums.len() >= 4 => steps.push(Step::RuneVel(nums[0] as u8, Vec3::new(nums[1], nums[3], nums[2]))),
            "sunrise" => steps.push(Step::Sunrise(nums.first().copied().unwrap_or(1.0))),
            "turn" if nums.len() >= 3 => steps.push(Step::Turn(nums[0], nums[1], nums[2] as u32)),
            "burst" => {
                let (name, n) = arg.split_once(',').unwrap_or((arg, "60"));
                steps.push(Step::Burst(name.to_string(), n.trim().parse().unwrap_or(60)));
            }
            "quit" => steps.push(Step::Quit),
            "" => {}
            other => warn!("unknown debug step {other}"),
        }
    }
    Some(DebugScript { steps, ..default() })
}

/// Advances the script by one frame: continues the steps in progress (`turn` and `burst` alongside
/// the rest; `fwd` and `walk` hold the script until they finish), then starts the next step.
/// Runs before the player and grab systems, so a scripted key press is seen the same frame.
pub fn run_script(
    mut commands: Commands,
    script: Option<ResMut<DebugScript>>,
    mut exit: MessageWriter<AppExit>,
    mut player: Query<(&mut Transform, &mut PortalPawn, &mut PortalTraveller, &mut LinearVelocity)>,
    mut space: ResMut<PlayerSpace>,
    mut rune_vel: Query<(&super::rune::Rune, &mut LinearVelocity, &mut Transform, Option<&mut Position>, &PortalTraveller), Without<PortalPawn>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut ending: ResMut<super::Ending>,
) {
    let Some(mut s) = script else { return };
    // Let go of last frame's presses. Without this a key stays held, and pressing it again later
    // never counts as `just_pressed`.
    for k in std::mem::take(&mut s.pressed_keys) {
        keys.release(k);
    }
    if std::mem::take(&mut s.pressed_click) {
        mouse.release(MouseButton::Left);
    }
    if let Some((dyaw, dpitch, frames)) = s.turn {
        if let Ok((_, mut pawn, _, _)) = player.single_mut() {
            pawn.yaw = (pawn.yaw - dyaw.to_radians()).rem_euclid(std::f32::consts::TAU);
            pawn.pitch = (pawn.pitch + dpitch.to_radians()).clamp(-1.5, 1.5);
        }
        s.turn = (frames > 1).then_some((dyaw, dpitch, frames - 1));
    }
    if let Some((name, i, n)) = s.burst.clone() {
        let _ = std::fs::create_dir_all("shots");
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("shots/{name}_{i:04}.png")));
        s.burst = (i + 1 < n).then_some((name, i + 1, n));
    }
    if let Some((speed, frames)) = s.forward {
        if let Ok((_, pawn, _, mut lv)) = player.single_mut() {
            let v = Quat::from_rotation_y(pawn.yaw) * Vec3::NEG_Z * speed * pawn.scale;
            lv.x = v.x;
            lv.z = v.z;
        }
        s.forward = (frames > 1).then_some((speed, frames - 1));
        return;
    }
    if let Some((v, frames)) = s.walk {
        if let Ok((_, _, _, mut lv)) = player.single_mut() {
            lv.x = v.x;
            lv.z = v.z;
        }
        s.walk = if frames > 1 { Some((v, frames - 1)) } else { None };
        return;
    }
    if s.wait > 0 {
        s.wait -= 1;
        return;
    }
    let Some(step) = s.steps.get(s.next).cloned() else { return };
    s.next += 1;
    match step {
        Step::Wait(n) => s.wait = n,
        Step::Teleport { loc, yaw, pitch, space: sp } => {
            if let Ok((mut t, mut pawn, mut tr, mut lv)) = player.single_mut() {
                t.translation = ue::pos(loc);
                pawn.yaw = ue::view_yaw(yaw);
                pawn.pitch = pitch.to_radians();
                tr.reset_tracking();
                lv.0 = Vec3::ZERO;
            }
            space.0 = (sp >= 0).then_some(sp);
        }
        Step::Walk { v, frames } => s.walk = Some((v, frames)),
        Step::Forward(speed, frames) => s.forward = Some((speed, frames)),
        Step::Shot(name) => {
            let _ = std::fs::create_dir_all("shots");
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("shots/{name}.png")));
            s.wait = 6; // let the file be written
        }
        Step::Pos => {
            if let Ok((t, pawn, tr, lv)) = player.single_mut() {
                let p = t.translation * 100.0;
                for (r, _, rt, _, rtr) in rune_vel.iter() {
                    let p = rt.translation * 100.0;
                    info!("RUNE{} ue=({:.0}, {:.0}, {:.0}) scale={:.3} mirrored={}", r.index, p.x, p.z, p.y, rt.scale.y, rtr.mirrored);
                }
                info!("PLAYER ue=({:.0}, {:.0}, {:.0}) scale={:.3} mirrored={} traversals={} vel=({:.2},{:.2},{:.2}) space={:?}", p.x, p.z, p.y, pawn.scale, tr.mirrored, tr.traversal_count, lv.x, lv.z, lv.y, space.0);
            }
        }
        Step::RuneVel(i, v) => {
            for (r, mut lv, _, _, _) in &mut rune_vel {
                if r.index == i {
                    lv.0 = v;
                }
            }
        }
        Step::RuneTp(i, loc) => {
            for (r, mut lv, mut t, pos, _) in &mut rune_vel {
                if r.index == i {
                    t.translation = ue::pos(loc);
                    if let Some(mut p) = pos {
                        p.0 = t.translation;
                    }
                    lv.0 = Vec3::ZERO;
                }
            }
        }
        Step::Key(k) => {
            keys.press(k);
            s.pressed_keys.push(k);
        }
        Step::Click => {
            mouse.press(MouseButton::Left);
            s.pressed_click = true;
        }
        Step::Sunrise(a) => {
            ending.broken = true;
            ending.sunrise = a.clamp(0.0, 1.0);
        }
        Step::Turn(y, p, n) => s.turn = Some((y, p, n)),
        Step::Burst(name, n) => s.burst = Some((name, 0, n)),
        Step::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

/// `HEXFOLD_PIPELINES` (render world): logs every render pipeline the frame it is queued.
pub fn log_new_pipelines(cache: Res<bevy::render::render_resource::PipelineCache>, mut seen: Local<usize>, mut frame: Local<u32>) {
    use bevy::material::descriptor::PipelineDescriptor;
    *frame += 1;
    let all: Vec<_> = cache.pipelines().collect();
    for (i, p) in all.iter().enumerate().skip(*seen) {
        let (label, defs) = match &p.descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(d) => (d.label.clone(), d.fragment.as_ref().map(|f| format!("{:?}", f.shader_defs)).unwrap_or_default()),
            PipelineDescriptor::ComputePipelineDescriptor(d) => (d.label.clone(), String::new()),
            PipelineDescriptor::MeshPipelineDescriptor(d) => (d.label.clone(), String::new()),
        };
        log(&format!("PIPELINE f={} #{i} {:?} {defs}", *frame, label));
    }
    *seen = all.len();
}

/// `HEXFOLD_TRACE`: one line per frame with the player's motion and the portal view count, and
/// the player's contacts whose normal isn't straight up.
pub fn trace(
    time: Res<Time>,
    stats: Res<crate::portal::PortalStats>,
    space: Res<PlayerSpace>,
    player: Query<(&Transform, &PortalPawn, &PortalTraveller, &LinearVelocity, &super::player::Player)>,
    mut frame: Local<u32>,
    graph: Res<ContactGraph>,
    me: Query<Entity, With<super::player::Player>>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
) {
    *frame += 1;
    let Ok((t, pawn, tr, lv, p)) = player.single() else { return };
    if let Ok(e) = me.single() {
        for pair in graph.contact_pairs_with(e) {
            let other = if pair.collider1 == e { pair.collider2 } else { pair.collider1 };
            let sign = if pair.collider1 == e { -1.0 } else { 1.0 };
            for m in &pair.manifolds {
                let n = m.normal * sign;
                if n.y < 0.99 {
                    let root = parents.iter_ancestors(other).last().unwrap_or(other);
                    log(&format!("  CONTACT f={} with {:?}/{:?} normal(ue)=({:.2},{:.2},{:.2}) points={}", *frame, names.get(root).ok(), names.get(other).ok(), n.x, n.z, n.y, m.points.len()));
                }
            }
        }
    }
    let u = t.translation * 100.0;
    log(&format!(
        "TRACE f={} dt={:.2}ms ue=({:.1},{:.1},{:.2}) vel=({:.2},{:.2},{:.3}) scale={:.3} trav={} views={} near={:.4} space={:?} gnd={}",
        *frame, time.delta_secs() * 1000.0, u.x, u.z, u.y, lv.x, lv.z, lv.y, pawn.scale, tr.traversal_count, stats.views, stats.near_plane, space.0, p.grounded
    ));
}
