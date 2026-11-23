//! A scripted driver for automated checks: `HEXFOLD_SCRIPT="wait:60;tp:0,2000,100,-90,0,0;shot:a;quit"`.
//!
//! Steps (separated by `;`):
//! * `wait:N`            do nothing for N frames
//! * `tp:x,y,z,yaw,pitch,space`  teleport the player (Unreal cm and degrees; space -1 = none)
//! * `walk:dx,dy,dz,frames`  push the player with a velocity (Unreal axes, m/s) for some frames
//! * `shot:name`         save `shots/name.png`
//! * `quit`              exit
//!
//! Without the variable this does nothing.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::portal::{PlayerSpace, PortalPawn, PortalTraveller};
use crate::ue;

#[derive(Debug, Clone)]
enum Step {
    Wait(u32),
    Teleport { loc: [f32; 3], yaw: f32, pitch: f32, space: i32 },
    Walk { v: Vec3, frames: u32 },
    Shot(String),
    Pos,
    RuneVel(u8, Vec3),
    RuneTp(u8, [f32; 3]),
    Key(KeyCode),
    Click,
    /// Jump the ending to this progress (0 = night, 1 = dawn).
    Sunrise(f32),
    Quit,
}

#[derive(Resource, Default)]
pub struct DebugScript {
    steps: Vec<Step>,
    next: usize,
    wait: u32,
    walk: Option<(Vec3, u32)>,
}

pub fn load_script() -> Option<DebugScript> {
    let text = std::env::var("HEXFOLD_SCRIPT").ok()?;
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
            "shot" => steps.push(Step::Shot(arg.to_string())),
            "pos" => steps.push(Step::Pos),
            "key" => match arg { "E" => steps.push(Step::Key(KeyCode::KeyE)), "Space" => steps.push(Step::Key(KeyCode::Space)), _ => {} },
            "click" => steps.push(Step::Click),
            "runetp" if nums.len() >= 4 => steps.push(Step::RuneTp(nums[0] as u8, [nums[1], nums[2], nums[3]])),
            "runevel" if nums.len() >= 4 => steps.push(Step::RuneVel(nums[0] as u8, Vec3::new(nums[1], nums[3], nums[2]))),
            "sunrise" => steps.push(Step::Sunrise(nums.first().copied().unwrap_or(1.0))),
            "quit" => steps.push(Step::Quit),
            "" => {}
            other => warn!("unknown debug step {other}"),
        }
    }
    Some(DebugScript { steps, ..default() })
}

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
        Step::Key(k) => keys.press(k),
        Step::Click => mouse.press(MouseButton::Left),
        Step::Sunrise(a) => {
            ending.broken = true;
            ending.sunrise = a.clamp(0.0, 1.0);
        }
        Step::Quit => {
            exit.write(AppExit::Success);
        }
    }
}
