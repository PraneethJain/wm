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
        layout::LayoutStack,
        Config, WindowManager,
    },
    extensions::{actions::toggle_fullscreen, hooks::add_ewmh_hooks},
    map, stack,
    x11rb::RustConn,
    Result,
};
use std::collections::HashMap;
use tracing_subscriber::{self, prelude::*};

const WHITE: u32 = 0xffffffff;

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
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-S-Up" => send_layout_message(|| IncMain(1)),
        "M-S-Down" => send_layout_message(|| IncMain(-1)),
        "M-S-Right" => send_layout_message(|| ExpandMain),
        "M-S-Left" => send_layout_message(|| ShrinkMain),
        "M-f" => toggle_fullscreen(),
        "M-space" => toggle_floating_focused(),
        "M-S-q" => exit(),

        "M-p" => spawn("dmenu_run"),
        "M-Return" => spawn("alacritty"),
        "M-c" => spawn("emacs"),
        "M-b" => spawn("thorium"),
        "M-l" => spawn("xsecurelock"),
        "M-S-s" => spawn("flameshot gui"),
        "M-S-c" => spawn("xcolor -s clipboard"),
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
    let config = add_ewmh_hooks(Config {
        default_layouts: layouts(),
        focused_border: WHITE.into(),
        ..Config::default()
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
