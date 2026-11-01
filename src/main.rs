
use evdev::{
    AttributeSet, Device, EventSummary, EventType, InputEvent, KeyCode,
    uinput::VirtualDevice,
};
use gtk4 as gtk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use glib_unix::{unix_fd_add_local, unix_signal_add_local};

use directories::BaseDirs;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    os::fd::AsRawFd,
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

const HOLD_TIME: Duration = Duration::from_millis(450);
//test
// const TEST_DURATION: Duration = Duration::from_secs(45);

#[derive(Clone, Copy)]
struct Candidate {
    key: KeyCode,
    token: u64,
    started: Instant,
    long_triggered: bool,
    selected: bool,
    selecting: bool,
    forwarded: bool,
}

struct AppState {
    physical: Device,
    virtual_device: VirtualDevice,
    window: gtk::ApplicationWindow,
    title: gtk::Label,
    characters: gtk::Box,
    candidate: Option<Candidate>,
    popup_token: Option<u64>,
escape_suppressed: bool,
popup_suppressed_key: Option<KeyCode>,
popup_key: Option<KeyCode>,
highlighted: usize,
accent_buttons: Vec<gtk::Button>,
    accents: HashMap<KeyCode, Vec<String>>,
    shift_pressed: bool,
    caps_lock: bool,
ctrl_pressed: bool,
alt_pressed: bool,
super_pressed: bool,
    next_token: u64,
}

fn keycode_from_char(character: char) -> Option<KeyCode> {
    match character.to_ascii_lowercase() {
        'a' => Some(KeyCode::KEY_A),
        'b' => Some(KeyCode::KEY_B),
        'c' => Some(KeyCode::KEY_C),
        'd' => Some(KeyCode::KEY_D),
        'e' => Some(KeyCode::KEY_E),
        'f' => Some(KeyCode::KEY_F),
        'g' => Some(KeyCode::KEY_G),
        'h' => Some(KeyCode::KEY_H),
        'i' => Some(KeyCode::KEY_I),
        'j' => Some(KeyCode::KEY_J),
        'k' => Some(KeyCode::KEY_K),
        'l' => Some(KeyCode::KEY_L),
        'm' => Some(KeyCode::KEY_M),
        'n' => Some(KeyCode::KEY_N),
        'o' => Some(KeyCode::KEY_O),
        'p' => Some(KeyCode::KEY_P),
        'q' => Some(KeyCode::KEY_Q),
        'r' => Some(KeyCode::KEY_R),
        's' => Some(KeyCode::KEY_S),
        't' => Some(KeyCode::KEY_T),
        'u' => Some(KeyCode::KEY_U),
        'v' => Some(KeyCode::KEY_V),
        'w' => Some(KeyCode::KEY_W),
        'x' => Some(KeyCode::KEY_X),
        'y' => Some(KeyCode::KEY_Y),
        'z' => Some(KeyCode::KEY_Z),

        '1' => Some(KeyCode::KEY_1),
        '2' => Some(KeyCode::KEY_2),
        '3' => Some(KeyCode::KEY_3),
        '4' => Some(KeyCode::KEY_4),
        '5' => Some(KeyCode::KEY_5),
        '6' => Some(KeyCode::KEY_6),
        '7' => Some(KeyCode::KEY_7),
        '8' => Some(KeyCode::KEY_8),
        '9' => Some(KeyCode::KEY_9),

        _ => None,
    }
}

fn find_keyboard() -> Result<Device, String> {
    for (path, device) in evdev::enumerate() {
        let Some(name) = device.name() else {
            continue;
        };

        // Ignore virtual and special-purpose devices.
        let lower_name = name.to_lowercase();

        if lower_name.contains("virtual")
            || lower_name.contains("mouse")
            || lower_name.contains("touchpad")
            || lower_name.contains("hotkey")
            || lower_name.contains("button")
            || lower_name.contains("video")
            || lower_name.contains("wmi")
        {
            continue;
        }

        let Some(keys) = device.supported_keys() else {
            continue;
        };

        // Require a full set of ordinary keyboard keys.
        let required_keys = [
            evdev::KeyCode::KEY_A,
            evdev::KeyCode::KEY_Z,
            evdev::KeyCode::KEY_SPACE,
            evdev::KeyCode::KEY_ENTER,
            evdev::KeyCode::KEY_1,
            evdev::KeyCode::KEY_0,
            evdev::KeyCode::KEY_LEFTSHIFT,
            evdev::KeyCode::KEY_BACKSPACE,
        ];

        if required_keys.iter().all(|key| keys.contains(*key)) {
            println!(
                "Selected keyboard: {} ({})",
                name,
                path.display()
            );

            return Ok(device);
        }
    }

    Err("No suitable physical keyboard found.".to_string())
}

fn config_dir() -> std::path::PathBuf {
    let base_dirs = BaseDirs::new()
        .expect("Could not determine user directories");

    let config_dir = base_dirs.config_dir().join("accentpop");

    fs::create_dir_all(&config_dir)
        .expect("Could not create AccentPop configuration directory");

    config_dir
}

fn ensure_config_files() -> std::path::PathBuf {
    let dir = config_dir();

    let defaults = [
        ("accents.txt", include_str!("../resources/accents.txt")),
        ("style.css", include_str!("../resources/style.css")),
    ];

    for (filename, contents) in defaults {
        let path = dir.join(filename);

        if !path.exists() {
            if let Err(error) = fs::write(&path, contents) {
                eprintln!("Could not create {filename}: {error}");
            } else {
                println!("Created default configuration: {}", path.display());
            }
        }
    }

    dir
}

fn load_accents() -> HashMap<KeyCode, Vec<String>> {
    let mut accents = HashMap::new();

let accents_path = ensure_config_files().join("accents.txt");

let contents = match fs::read_to_string(&accents_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("Could not load accents.txt: {error}");
            return accents;
        }
    };

    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key_text, values_text)) = line.split_once('=') else {
            eprintln!("Invalid configuration at line {}", line_number + 1);
            continue;
        };

        let mut key_chars = key_text.trim().chars();

        let Some(character) = key_chars.next() else {
            continue;
        };

        if key_chars.next().is_some() {
            eprintln!("Invalid key at line {}", line_number + 1);
            continue;
        }

        let Some(key) = keycode_from_char(character) else {
            eprintln!("Unsupported key at line {}", line_number + 1);
            continue;
        };

        let values: Vec<String> = values_text
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from)
            .collect();

        if !values.is_empty() {
            accents.insert(key, values);
        }
    }

    println!("Loaded {} accent key definitions.", accents.len());

    accents
}

fn is_accent_key(state: &AppState, key: KeyCode) -> bool {
    state.accents.contains_key(&key)
}

fn apply_case(character: &str, shift: bool, caps_lock: bool) -> String {
    if shift ^ caps_lock {
        character.to_uppercase()
    } else {
        character.to_string()
    }
}

fn cursor_pos() -> (i32, i32) {
    if let Ok(output) = Command::new("hyprctl")
        .arg("cursorpos")
        .output()
    {
        if let Ok(text) = std::str::from_utf8(&output.stdout) {
            let mut parts = text.trim().splitn(2, ", ");

            if let (Some(xs), Some(ys)) = (parts.next(), parts.next()) {
                if let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>()) {
                    return (x, y);
                }
            }
        }
    }

    eprintln!("Could not read cursor position; using screen center.");
    (960, 540)
}

fn monitor_at_cursor(
    cursor_x: i32,
    cursor_y: i32,
) -> Option<(gtk::gdk::Monitor, gtk::gdk::Rectangle)> {
    let display = gtk::gdk::Display::default()?;
    let monitors = display.monitors();

    let mut fallback = None;

    for index in 0..monitors.n_items() {
        let monitor = monitors
            .item(index)?
            .downcast::<gtk::gdk::Monitor>()
            .ok()?;

        let geometry = monitor.geometry();

        if fallback.is_none() {
            fallback = Some((monitor.clone(), geometry));
        }

        if cursor_x >= geometry.x()
            && cursor_x < geometry.x() + geometry.width()
            && cursor_y >= geometry.y()
            && cursor_y < geometry.y() + geometry.height()
        {
            return Some((monitor, geometry));
        }
    }

    fallback
}

fn show_popup(state: &Rc<RefCell<AppState>>, key: KeyCode, token: u64) {
{
    let mut state = state.borrow_mut();
    state.popup_token = Some(token);
    state.popup_key = Some(key);
    state.highlighted = 0;
    state.accent_buttons.clear();
}
    let (window, title, characters) = {
        let state = state.borrow();
        (
            state.window.clone(),
            state.title.clone(),
            state.characters.clone(),
        )
    };

    while let Some(child) = characters.first_child() {
        characters.remove(&child);
    }

let key_debug = format!("{:?}", key);
let key_name = key_debug.strip_prefix("KEY_").unwrap_or("?");

title.set_text(&format!("Choose an accent key for {}", key_name));

title.set_text(&format!("Choose an accent key for {}", key_name));

let accent_list = state
    .borrow()
    .accents
    .get(&key)
    .cloned()
    .unwrap_or_default();

for (index, character) in accent_list.iter().enumerate() {
    let selected = {
        let state = state.borrow();
        apply_case(character, state.shift_pressed, state.caps_lock)
    };

    let button = gtk::Button::with_label(&selected);
    button.add_css_class("accent-button");

    if index == 0 {
        button.add_css_class("accent-selected");
    }

    state.borrow_mut().accent_buttons.push(button.clone());

        let click_state = state.clone();

        button.connect_clicked(move |_| {
            let valid = {
    let mut state = click_state.borrow_mut();

    if state.popup_token == Some(token) {
    state.popup_token = None;
    state.popup_key = None;
    true
} else {
        false
    }
};

            if valid {
    match Command::new("wtype")
        .arg(&selected)
        .status()
    {
        Ok(status) if status.success() => {
            println!("Inserted accent: {}", selected);
        }
        Ok(status) => {
            eprintln!("wtype exited with status: {}", status);
        }
        Err(error) => {
            eprintln!("Could not execute wtype: {}", error);
        }
    }

    click_state.borrow().window.hide();
}
        });

        characters.append(&button);
    }

    let (cursor_x, cursor_y) = cursor_pos();

if let Some((monitor, geometry)) =
    monitor_at_cursor(cursor_x, cursor_y)
{
    let screen_width = geometry.width();
    let screen_height = geometry.height();

    let local_x = cursor_x - geometry.x();
    let local_y = cursor_y - geometry.y();

    let popup_width = 330;
    let popup_height = 100;
    let spacing = 20;

    let popup_x = (local_x - popup_width / 2)
        .clamp(0, screen_width - popup_width);

    let popup_y = if local_y + spacing + popup_height <= screen_height {
        local_y + spacing
    } else {
        local_y - spacing - popup_height
    }
    .clamp(0, screen_height - popup_height);

    window.set_monitor(Some(&monitor));
    window.set_margin(Edge::Left, popup_x);
    window.set_margin(Edge::Top, popup_y);
} else {
    eprintln!("Could not detect a monitor for the popup.");
}

window.present();
}

fn commit_highlight(state: &Rc<RefCell<AppState>>) {
    let selected = {
        let mut state = state.borrow_mut();

        if state.popup_token.is_none() {
            return;
        }

        let key = match state.popup_key {
            Some(key) => key,
            None => return,
        };

let accents = match state.accents.get(&key) {
    Some(accents) if !accents.is_empty() => accents,
    _ => return,
};

let index = state.highlighted.min(accents.len() - 1);
let selected = apply_case(
    &accents[index],
    state.shift_pressed,
    state.caps_lock,
);

        state.popup_token = None;
        state.popup_key = None;
        state.window.hide();

        selected
    };

    match Command::new("wtype").arg(&selected).status() {
        Ok(status) if status.success() => {
            println!("Inserted accent: {}", selected);
        }
        Ok(status) => {
            eprintln!("wtype exited with status: {}", status);
        }
        Err(error) => {
            eprintln!("Could not execute wtype: {}", error);
        }
    }
}

fn main() {
    let app = gtk::Application::builder()
        .application_id("io.accentpop")
        .build();

    app.connect_activate(|app| {
        // Open the physical keyboard.
let physical = find_keyboard()
    .expect("Could not find a suitable physical keyboard");

        // Create a virtual keyboard with the same supported keys.
        let mut keys = AttributeSet::<KeyCode>::new();

        if let Some(supported) = physical.supported_keys() {
            for key in supported.iter() {
                keys.insert(key);
            }
        }

        let virtual_device = VirtualDevice::builder()
            .expect("Could not create virtual keyboard")
            .name("AccentPop Virtual Keyboard")
            .with_keys(&keys)
            .expect("Could not configure virtual keyboard")
            .build()
            .expect("Could not start virtual keyboard");

        physical
            .set_nonblocking(true)
            .expect("Could not set keyboard nonblocking");

        // Create the popup.
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("AccentPop")
            .default_width(330)
            .default_height(100)
            .decorated(false)
            .resizable(false)
            .build();

let provider = gtk::CssProvider::new();
let css_path = ensure_config_files().join("style.css");
provider.load_from_path(&css_path);

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("No display found"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

window.init_layer_shell();
window.set_layer(Layer::Overlay);
window.set_anchor(Edge::Left, true);
window.set_anchor(Edge::Top, true);
window.set_anchor(Edge::Right, false);
window.set_anchor(Edge::Bottom, false);
window.set_keyboard_mode(KeyboardMode::None);

        let container = gtk::Box::new(
            gtk::Orientation::Vertical,
            10,
        );

        container.set_margin_top(14);
        container.set_margin_bottom(14);
        container.set_margin_start(16);
        container.set_margin_end(16);

        let title = gtk::Label::new(Some("Choose an accent"));
        title.add_css_class("heading");

        let characters = gtk::Box::new(
            gtk::Orientation::Horizontal,
            8,
        );

        characters.set_homogeneous(true);

        container.append(&title);
        container.append(&characters);
        window.set_child(Some(&container));

        // Initialize shared application state.
        let state = Rc::new(RefCell::new(AppState {
            physical,
            virtual_device,
            window: window.clone(),
            title,
            characters,
            candidate: None,
	popup_token: None,
	escape_suppressed: false,
popup_suppressed_key: None,
popup_key: None,
highlighted: 0,
accent_buttons: Vec::new(),
            accents: load_accents(),
            shift_pressed: false,
ctrl_pressed: false,
alt_pressed: false,
super_pressed: false,
            caps_lock: false,
            next_token: 0,
        }));

        // Grab the physical keyboard only after the virtual
        // keyboard and UI have been initialized.
        state.borrow_mut().physical
            .grab()
            .expect("Could not grab physical keyboard");

        // Release the physical keyboard when AccentPop shuts down.
        let shutdown_state = state.clone();

        app.connect_shutdown(move |_| {
            println!("AccentPop shutting down. Releasing keyboard.");
            let _ = shutdown_state.borrow_mut().physical.ungrab();
        });

// Handle SIGTERM from systemd.
let sigterm_app = app.clone();

unix_signal_add_local(libc::SIGTERM, move || {
    println!("Received SIGTERM. Shutting down AccentPop.");
    sigterm_app.quit();
    gtk::glib::ControlFlow::Break
});

// Handle SIGINT from Ctrl+C.
let sigint_app = app.clone();

unix_signal_add_local(libc::SIGINT, move || {
    println!("Received SIGINT. Shutting down AccentPop.");
    sigint_app.quit();
    gtk::glib::ControlFlow::Break
});

        println!("AccentPop keyboard integration started.");
        println!("Hold A, E or O for 450 ms.");
        println!("Other keys pass through normally.");
       // println!("Automatic safety release in 45 seconds.");

        // Hide the popup until a long press occurs.
        window.present();
        window.hide();

        // Register the physical keyboard with the GTK main loop.
        let fd = state.borrow().physical.as_raw_fd();
        let event_state = state.clone();
unix_fd_add_local(
            fd,
            gtk::glib::IOCondition::IN
                | gtk::glib::IOCondition::HUP
                | gtk::glib::IOCondition::ERR,
            move |_, _| {
                // Read available keyboard events.
          let read_result = {
    let mut state = event_state.borrow_mut();

    state.physical.fetch_events().map(|events| {
        events
            .filter_map(|event| {
                if let EventSummary::Key(_, key, value) =
                    event.destructure()
                {
                    Some((key, value))
                } else {
                    None
                }
            })
            .collect::<Vec<(KeyCode, i32)>>()
    })
};

let key_events = match read_result {
    Ok(events) => events,

    Err(error)
        if error.kind() == std::io::ErrorKind::WouldBlock =>
    {
        return gtk::glib::ControlFlow::Continue;
    }

    Err(error) => {
        eprintln!("Keyboard input error: {error}");

        let _ = event_state.borrow_mut().physical.ungrab();

        return gtk::glib::ControlFlow::Break;
    }
};

                let mut forwarded = Vec::new();
                let mut new_timer = None;

                for (key, value) in key_events {
    let mut state = event_state.borrow_mut();

// Track physical keyboard modifiers.
match key {
    KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => {
        state.shift_pressed = value != 0;
    }

    KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => {
        state.ctrl_pressed = value != 0;
    }

    KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => {
        state.alt_pressed = value != 0;
    }

    KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => {
        state.super_pressed = value != 0;
    }

    KeyCode::KEY_CAPSLOCK if value == 1 => {
        state.caps_lock = !state.caps_lock;
        println!("Caps Lock: {}", state.caps_lock);
    }

    _ => {}
}

    // Suppress Escape release if its press dismissed the popup.
    if state.escape_suppressed {
        if key == KeyCode::KEY_ESC && value == 0 {
            state.escape_suppressed = false;
        }
        continue;
    }
// Suppress release and repeat events for popup control keys.
if state.popup_suppressed_key == Some(key) {
    if value == 0 {
        state.popup_suppressed_key = None;
    }
    continue;
}

     // Handle keyboard input while the accent popup is open.
    if state.popup_token.is_some() && value == 1 {

// Arrow keys navigate the highlighted accent.
let direction = match key {
    KeyCode::KEY_LEFT | KeyCode::KEY_UP => Some(-1),
    KeyCode::KEY_RIGHT | KeyCode::KEY_DOWN => Some(1),
    _ => None,
};

if let Some(direction) = direction {
    state.popup_suppressed_key = Some(key);

let count = state
    .popup_key
    .and_then(|key| state.accents.get(&key))
    .map(|accents| accents.len())
    .unwrap_or(0);

    if count > 0 {
        state.highlighted = if direction < 0 {
            (state.highlighted + count - 1) % count
        } else {
            (state.highlighted + 1) % count
        };

        for (index, button) in
            state.accent_buttons.iter().enumerate()
        {
            button.remove_css_class("accent-selected");

            if index == state.highlighted {
                button.add_css_class("accent-selected");
            }
        }

if let Some(accents) = state
    .popup_key
    .and_then(|key| state.accents.get(&key))
{
    println!("Accent selection: {}", accents[state.highlighted]);
}
    }

    continue;
}

// Space or Enter confirms the highlighted accent.
if key == KeyCode::KEY_SPACE || key == KeyCode::KEY_ENTER {
    state.popup_suppressed_key = Some(key);

    println!("Confirming highlighted accent.");

    drop(state);
    commit_highlight(&event_state);

    continue;
}

        // Escape closes the popup without passing Escape through.
        if key == KeyCode::KEY_ESC {
            state.popup_token = None;
            state.popup_key = None;
            state.window.hide();
            state.escape_suppressed = true;

            println!("Popup dismissed with Escape.");
            continue;
        }

        // Pressing the same accent key starts a selection press.
        if state.popup_key == Some(key) {
            let token = state.next_token;
            state.next_token += 1;

            state.candidate = Some(Candidate {
                key,
                token,
                started: Instant::now(),
                long_triggered: false,
                selected: false,
                selecting: true,
	forwarded: false,
            });

            new_timer = Some((key, token));

            println!("Accent selection press started: {:?}", key);
            continue;
        }

        // Any other key dismisses the popup.
        state.popup_token = None;
        state.popup_key = None;
        state.window.hide();

        println!("Popup dismissed by another key.");
    }

// Handle an existing accent-key candidate.
if let Some(mut candidate) = state.candidate {
    if key == candidate.key {
        if value == 0 {
            if candidate.forwarded {
                // Forward the release of a key already sent.
                forwarded.push(InputEvent::new(
                    EventType::KEY.0,
                    key.0,
                    0,
                ));
            } else if candidate.selecting {
                // Short second press: cycle to the next accent.
                if !candidate.long_triggered
                    && state.popup_token.is_some()
                {
                    let popup_key = state.popup_key.unwrap();

                    let count = state
                        .accents
                        .get(&popup_key)
                        .map(|accents| accents.len())
                        .unwrap_or(0);

                    if count > 0 {
                        state.highlighted =
                            (state.highlighted + 1) % count;

                        for (index, button) in
                            state.accent_buttons.iter().enumerate()
                        {
                            button.remove_css_class("accent-selected");

                            if index == state.highlighted {
                                button.add_css_class("accent-selected");
                            }
                        }

                        if let Some(accents) =
                            state.accents.get(&popup_key)
                        {
                            println!(
                                "Accent selection: {}",
                                accents[state.highlighted]
                            );
                        }
                    }
                }
            } else if !candidate.long_triggered {
                // Original quick tap: reproduce the character.
                forwarded.push(InputEvent::new(
                    EventType::KEY.0,
                    key.0,
                    1,
                ));

                forwarded.push(InputEvent::new(
                    EventType::KEY.0,
                    key.0,
                    0,
                ));

                println!("Quick tap: {:?}", key);
            }

            state.candidate = None;
        }

        // Suppress repeats of the candidate key.
        continue;
    }

    // Another key was pressed while this candidate was pending.
    if value == 1
        && !candidate.long_triggered
        && !candidate.forwarded
        && !candidate.selecting
    {
        // Immediately forward the pending accent key.
        forwarded.push(InputEvent::new(
            EventType::KEY.0,
            candidate.key.0,
            1,
        ));

        candidate.forwarded = true;
        state.candidate = Some(candidate);

        // The new key will be handled normally below.
    } else if !candidate.forwarded && value == 0 {
        // An unrelated key was released before the candidate.
        forwarded.push(InputEvent::new(
            EventType::KEY.0,
            key.0,
            value,
        ));

        continue;
    } else {
        // Keep the candidate active for its original key.
        state.candidate = Some(candidate);
    }
}

// Start a new candidate only when no shortcut modifier is active.
if value == 1
    && state.candidate.is_none()
    && !state.ctrl_pressed
    && !state.alt_pressed
    && !state.super_pressed
    && is_accent_key(&state, key)
{
                        let token = state.next_token;
                        state.next_token += 1;

                        state.candidate = Some(Candidate {
    key,
    token,
    started: Instant::now(),
    long_triggered: false,
    selected: false,
    selecting: false,
    forwarded: false,
});

                        new_timer = Some((key, token));
                        println!("Waiting: {:?}", key);
                        continue;
                    }

                    // Forward all other keyboard events.
                    forwarded.push(InputEvent::new(
                        EventType::KEY.0,
                        key.0,
                        value,
                    ));
                }

                // Start a one-shot timer for the new candidate.
                if let Some((key, token)) = new_timer {
                    let timer_state = event_state.clone();

                    gtk::glib::timeout_add_local_once(
                        HOLD_TIME,
                        move || {
                           let action = {
    let mut state = timer_state.borrow_mut();

    if let Some(candidate) = state.candidate.as_mut() {
        if candidate.token == token && !candidate.long_triggered {
            candidate.long_triggered = true;
            Some(candidate.selecting)
        } else {
            None
        }
    } else {
        None
    }
};

match action {
    Some(true) => {
        // Second long press: commit the highlighted accent.
        println!("Long press: selecting highlighted accent.");
        commit_highlight(&timer_state);
    }

    Some(false) => {
        // First long press: open the popup.
        println!("LONG PRESS: {:?}", key);
        show_popup(&timer_state, key, token);
    }

    None => {}
}
                        },
                    );
                }

                // Forward collected events to the virtual keyboard.
                if !forwarded.is_empty() {
                    if let Err(error) =
                        event_state.borrow_mut().virtual_device.emit(&forwarded)
                    {
                        eprintln!("Virtual keyboard error: {error}");
                    }
                }

                gtk::glib::ControlFlow::Continue
            },
        );

        // Automatic safety release after 45 seconds.
     /*   let safety_state = state.clone();
     	 let safety_app = app.clone();

        gtk::glib::timeout_add_local_once(
            TEST_DURATION,
            move || {
                println!("Safety timeout reached. Releasing keyboard.");

                let _ = safety_state.borrow_mut().physical.ungrab();
                safety_app.quit();
            },
        );
*/
    });

    app.run();
}
