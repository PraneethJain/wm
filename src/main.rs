use penrose::{
    builtin::{
        actions::{
            exit,
            floating::{
                sink_focused, toggle_floating_focused, MouseDragHandler, MouseResizeHandler,
            },
            modify_with, send_layout_message, spawn,
        },
        layout::{
            messages::{ExpandMain, IncMain, ShrinkMain},
            transformers::Gaps,
            MainAndStack, Monocle,
        },
    },
    core::{
        bindings::{
            click_handler, parse_keybindings_with_xmodmap, KeyEventHandler, MouseEventHandler,
            MouseState,
        },
        hooks::EventHook,
        layout::LayoutStack,
        Config, State, WindowManager,
    },
    extensions::{
        actions::{focus_or_spawn, toggle_fullscreen},
        hooks::add_ewmh_hooks,
    },
    map, stack, util,
    x::{event, Atom, ClientConfig, Prop, XConn, XEvent},
    x11rb::RustConn,
    Result,
};
use std::collections::HashMap;
use tracing_subscriber::{self, prelude::*};

const WHITE: u32 = 0xffffffff;

#[derive(Debug, Clone, Default)]
pub struct FullScreenHook {
    fullscreen_border_px: u32,
}

impl<X: XConn> EventHook<X> for FullScreenHook {
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
        if let &XEvent::PropertyNotify(event::PropertyEvent { id, .. }) = &event {
            let net_wm_state = Atom::NetWmState.as_ref();
            let full_screen = x.intern_atom(Atom::NetWmStateFullscreen.as_ref())?;
            if let Ok(Some(Prop::Cardinal(vals))) = x.get_prop(*id, net_wm_state) {
                x.set_client_config(
                    *id,
                    &[ClientConfig::BorderPx(if vals.contains(&full_screen) {
                        self.fullscreen_border_px
                    } else {
                        state.config.border_width
                    })],
                )?;
            }
        }

        Ok(true)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MonitorHook {
    wallpaper_path: String,
}

impl<X: XConn> EventHook<X> for MonitorHook {
    fn call(&mut self, event: &XEvent, _: &mut State<X>, _: &X) -> Result<bool> {
        if let &XEvent::RandrNotify = &event {
            util::spawn(format!("feh --bg-max {} --no-fehbg", self.wallpaper_path))?;
        }

        Ok(true)
    }
}

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RustConn>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-Left" => modify_with(|cs| cs.focus_up()),
        "M-Up" => modify_with(|cs| cs.focus_up()),
        "M-Right" => modify_with(|cs| cs.focus_down()),
        "M-Down" => modify_with(|cs| cs.focus_down()),
        "M-S-k" => modify_with(|cs| cs.swap_down()),
        "M-S-j" => modify_with(|cs| cs.swap_up()),
        "M-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-S-Tab" => modify_with(|cs| {
            let focussed_screen_index = cs.current_screen().index();
            let unfocussed_screens = cs.screens().filter(|s| s.index() != focussed_screen_index).collect::<Vec<_>>();
            if let Some(unfocussed_screen) = unfocussed_screens.first() {
               cs.pull_tag_to_screen(cs.tag_for_screen(unfocussed_screen.index()).unwrap().to_string());
            }
        }),
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-S-Up" => send_layout_message(|| IncMain(1)),
        "M-S-Down" => send_layout_message(|| IncMain(-1)),
        "M-S-Right" => send_layout_message(|| ExpandMain),
        "M-S-Left" => send_layout_message(|| ShrinkMain),
        "M-f" =>   toggle_fullscreen(),
        "M-space" => toggle_floating_focused(),
        "M-S-q" => exit(),

        "M-p" => spawn("dmenu_run"),
        "M-c" => spawn("emacsclient -c"),
        "M-Return" => spawn("starteshell"),
        "M-d" => spawn("startdired"),
        "M-b" => spawn("thorium"),
        "M-v" => spawn("code"),
        "M-l" => spawn("xsecurelock"),
        "M-S-s" => spawn("flameshot gui"),
        "Print" => spawn("flameshot screen"),
        "M-S-c" => spawn("xcolor -s clipboard"),
        "M-s" => focus_or_spawn("spotify", "spotify"),

        "XF86AudioRaiseVolume" => spawn("pactl set-sink-volume @DEFAULT_SINK@ +5%"),
        "XF86AudioLowerVolume" => spawn("pactl set-sink-volume @DEFAULT_SINK@ -5%"),
        "XF86AudioMute" => spawn("pamixer -t"),
        "XF86MonBrightnessUp" => spawn("light -A 5"),
        "XF86MonBrightnessDown" => spawn("light -U 5"),
        "XF86AudioPlay" => spawn("dbus-send --print-reply --dest=org.mpris.MediaPlayer2.spotify /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.PlayPause"),
        "XF86AudioNext" => spawn("dbus-send --print-reply --dest=org.mpris.MediaPlayer2.spotify /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.Next"),
        "XF86AudioPrev" => spawn("dbus-send --print-reply --dest=org.mpris.MediaPlayer2.spotify /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.Previous"),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |client_set| client_set.focus_tag(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

fn mouse_bindings() -> HashMap<MouseState, Box<dyn MouseEventHandler<RustConn>>> {
    use penrose::core::bindings::{
        ModifierKey::{Meta, Shift},
        MouseButton::{Left, Middle, Right},
    };

    map! {
        map_keys: |(button, modifiers)| MouseState { button, modifiers };

        (Left, vec![Shift, Meta]) => MouseDragHandler::boxed_default(),
        (Right, vec![Shift, Meta]) => MouseResizeHandler::boxed_default(),
        (Middle, vec![Shift, Meta]) => click_handler(sink_focused()),
    }
}

fn layouts() -> LayoutStack {
    stack!(MainAndStack::boxed_default(), Monocle::boxed()).map(|layout| Gaps::wrap(layout, 10, 10))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .finish()
        .init();

    let conn = RustConn::new()?;
    let key_bindings = parse_keybindings_with_xmodmap(raw_key_bindings())?;
    let mut config = add_ewmh_hooks(Config {
        default_layouts: layouts(),
        focused_border: WHITE.into(),
        event_hook: Some(Box::new(FullScreenHook {
            fullscreen_border_px: 0,
        })),
        ..Config::default()
    });
    config.compose_or_set_event_hook(MonitorHook {
        wallpaper_path: "/home/praneeth/Pictures/wall5.jpg".to_string(),
    });
    let wm = WindowManager::new(config, key_bindings, mouse_bindings(), conn)?;

    wm.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_parse_correctly_with_xmodmap() {
        let res = parse_keybindings_with_xmodmap(raw_key_bindings());

        if let Err(e) = res {
            panic!("{e}");
        }
    }
}
