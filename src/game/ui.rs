//! The HUD (six rune slots, a crosshair, short messages) and the title, pause and escape screens.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::PrimaryWindow;

use super::AppState;
use super::hex::HexEvent;

/// A short message at the bottom of the screen ("The rune locks into place.").
#[derive(Message, Debug, Clone)]
pub struct Toast(pub String);

/// The HUD's root node.
#[derive(Component)]
pub struct Hud;
/// The HUD slot of rune `n`, lit gold when the rune is in the hex.
#[derive(Component)]
pub struct RuneSlot(pub u8);
/// The text of the message line.
#[derive(Component)]
pub struct ToastText;
/// The root of a menu screen (despawned when the screen closes).
#[derive(Component)]
pub struct MenuUi;

/// What a menu button does.
#[derive(Component, Clone, Copy, Debug)]
pub enum ButtonAction {
    /// Start playing (title screen).
    Play,
    /// Back to the game (pause screen).
    Resume,
    /// Start over: relaunches the executable without the title screen.
    Restart,
    /// Exit the game.
    Quit,
}

/// Seconds left before the current message has faded out.
#[derive(Resource, Default)]
pub struct ToastTimer(pub f32);

/// Titles, buttons and lit rune slots.
const GOLD: Color = Color::srgb(1.0, 0.82, 0.35);
/// Rune slots still to fill.
const DIM: Color = Color::srgba(0.45, 0.42, 0.35, 0.55);
/// The rune slots' labels (the Unreal HUD's icon textures weren't exported).
const NUMERALS: [&str; 6] = ["I", "II", "III", "IV", "V", "VI"];

/// A text node of `size` pixels.
fn text(s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(s.into()), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color))
}

/// Spawns the HUD: six rune slots at the top, a crosshair, and the message line.
pub fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Hud"),
            Hud,
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, ..default() },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            // Rune slots, top centre.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                for k in 0..6u8 {
                    row.spawn((
                        RuneSlot(k),
                        Node {
                            width: Val::Px(44.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(DIM),
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                    ))
                    .with_children(|c| {
                        c.spawn(text(NUMERALS[k as usize], 22.0, DIM));
                    });
                }
            });
            // Crosshair.
            root.spawn((
                Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), top: Val::Percent(50.0), width: Val::Px(4.0), height: Val::Px(4.0), margin: UiRect { left: Val::Px(-2.0), top: Val::Px(-2.0), ..default() }, ..default() },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
            ));
            // Message line.
            root.spawn(Node { position_type: PositionType::Absolute, bottom: Val::Px(70.0), width: Val::Percent(100.0), justify_content: JustifyContent::Center, ..default() })
                .with_children(|c| {
                    c.spawn((ToastText, text("", 24.0, Color::srgba(1.0, 0.95, 0.8, 0.0))));
                });
        });
}

/// Lights a rune's slot when the hex accepts it.
pub fn hud_events(
    mut events: MessageReader<HexEvent>,
    mut slots: Query<(&RuneSlot, &mut BorderColor, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    for e in events.read() {
        if let HexEvent::RuneLit(k) = e {
            for (slot, mut border, children) in &mut slots {
                if slot.0 == *k {
                    *border = BorderColor::all(GOLD);
                    for c in children.iter() {
                        if let Ok(mut t) = texts.get_mut(c) {
                            t.0 = GOLD;
                        }
                    }
                }
            }
        }
    }
}

/// Shows the latest [`Toast`] for 3.5 s, fading out over the last 0.6 s (in real time, so it also
/// fades on the pause screen).
pub fn toast_system(
    time: Res<Time<Real>>,
    mut messages: MessageReader<Toast>,
    mut timer: ResMut<ToastTimer>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<ToastText>>,
) {
    let Ok((mut text, mut color)) = text_q.single_mut() else { return };
    for m in messages.read() {
        text.0 = m.0.clone();
        timer.0 = 3.5;
    }
    timer.0 = (timer.0 - time.delta_secs()).max(0.0);
    let alpha = (timer.0 / 0.6).clamp(0.0, 1.0);
    color.0 = Color::srgba(1.0, 0.95, 0.8, alpha);
}

/// A full-screen menu: a title, an optional subtitle and a column of buttons.
fn spawn_menu(commands: &mut Commands, title: &str, subtitle: &str, buttons: &[(&str, ButtonAction)]) {
    commands
        .spawn((
            Name::new("Menu"),
            MenuUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.015, 0.04, 0.82)),
            GlobalZIndex(100),
        ))
        .with_children(|root| {
            root.spawn(text(title, 84.0, GOLD));
            if !subtitle.is_empty() {
                root.spawn(text(subtitle, 24.0, Color::srgb(0.8, 0.8, 0.85)));
            }
            for (label, action) in buttons {
                root.spawn((
                    Button,
                    *action,
                    Node { width: Val::Px(260.0), height: Val::Px(56.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(2.0)), ..default() },
                    BorderColor::all(GOLD),
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.16, 0.9)),
                ))
                .with_children(|b| {
                    b.spawn(text(*label, 28.0, GOLD));
                });
            }
        });
}

/// The title screen: Play, Quit.
pub fn open_title(mut commands: Commands) {
    spawn_menu(&mut commands, "HEXFOLD", "Find the six runes. The Observatory is bigger inside.", &[("Play", ButtonAction::Play), ("Quit", ButtonAction::Quit)]);
}

/// The pause screen: Resume, Restart, Quit.
pub fn open_pause(mut commands: Commands) {
    spawn_menu(&mut commands, "PAUSED", "", &[("Resume", ButtonAction::Resume), ("Restart", ButtonAction::Restart), ("Quit", ButtonAction::Quit)]);
}

/// The end screen: Play Again, Quit.
pub fn open_escaped(mut commands: Commands) {
    spawn_menu(&mut commands, "YOU ESCAPED", "The seal is broken, and the stars are out.", &[("Play Again", ButtonAction::Restart), ("Quit", ButtonAction::Quit)]);
}

/// Removes every open menu screen.
pub fn close_menus(mut commands: Commands, menus: Query<Entity, With<MenuUi>>) {
    for e in &menus {
        commands.entity(e).despawn();
    }
}

/// A menu button was pressed.
pub fn on_button(ev: On<Activate>, actions: Query<&ButtonAction>, mut next: ResMut<NextState<AppState>>, mut exit: MessageWriter<AppExit>) {
    let Ok(action) = actions.get(ev.entity) else { return };
    match action {
        ButtonAction::Play | ButtonAction::Resume => {
            next.set(AppState::Playing);
        }
        ButtonAction::Quit => {
            exit.write(AppExit::Success);
        }
        ButtonAction::Restart => restart(&mut exit),
    }
}

/// Starts a fresh copy of the game and closes this one.
fn restart(exit: &mut MessageWriter<AppExit>) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).env("HEXFOLD_SKIP_TITLE", "1").spawn();
    }
    exit.write(AppExit::Success);
}

/// Escape pauses and resumes. On the web it only pauses (and usually never arrives: the browser
/// uses it to release the pointer lock, and `player::grab_cursor` pauses on that). A browser
/// grants a pointer lock only after a click, so the player resumes with the Resume button.
pub fn pause_key(keys: Res<ButtonInput<KeyCode>>, state: Res<State<AppState>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::Playing => next.set(AppState::Paused),
            AppState::Paused if !cfg!(target_arch = "wasm32") => next.set(AppState::Playing),
            _ => {}
        }
    }
}

/// The world stands still on the title, pause and escape screens.
pub fn freeze_time(state: Res<State<AppState>>, mut time: ResMut<Time<Virtual>>) {
    if *state.get() == AppState::Playing {
        if time.is_paused() {
            time.unpause();
        }
    } else if !time.is_paused() {
        time.pause();
    }
}

/// The window size the HUD and menus were laid out for.
const UI_DESIGN_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

/// Scales the HUD and menus with the window, so a 1280 x 720 layout fits any window (a small
/// itch.io frame, a phone-sized browser, a 4K screen) without overflowing or looking tiny.
pub fn fit_ui_scale(window: Single<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let size = window.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let fit = (size / UI_DESIGN_SIZE).min_element().clamp(0.25, 4.0);
    if (scale.0 - fit).abs() > 1e-3 {
        scale.0 = fit;
    }
}
