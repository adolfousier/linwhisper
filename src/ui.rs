use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::audio::Recorder;
use crate::config::Config;
use crate::db::Db;

const CSS: &str = r#"
    window {
        background-color: transparent;
    }
    .mic-button {
        min-width: 52px;
        min-height: 52px;
        border-radius: 50%;
        background-color: #4a90d9;
        color: white;
        font-size: 22px;
        border: none;
        box-shadow: 0 2px 8px rgba(0,0,0,0.3);
        transition: background-color 200ms;
    }
    .mic-button:hover {
        background-color: #5ba0e9;
    }
    .mic-button.recording {
        background-color: #e04040;
        animation: pulse 1s ease-in-out infinite;
    }
    @keyframes pulse {
        0%   { opacity: 1.0; }
        50%  { opacity: 0.6; }
        100% { opacity: 1.0; }
    }
    .status-label {
        color: white;
        font-size: 10px;
        background-color: rgba(0,0,0,0.6);
        border-radius: 4px;
        padding: 2px 6px;
    }
"#;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    Recording,
    Processing,
}

pub fn build_ui(app: &gtk4::Application, config: Arc<Config>) {
    // Load CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(CSS);
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("LinWhisper")
        .default_width(64)
        .default_height(80)
        .decorated(false)
        .resizable(false)
        .build();

    // Layout
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vbox.set_halign(gtk4::Align::Center);
    vbox.set_valign(gtk4::Align::Center);

    let button = gtk4::Button::with_label("\u{1F3A4}");
    button.add_css_class("mic-button");
    button.set_size_request(52, 52);

    let status = gtk4::Label::new(None);
    status.add_css_class("status-label");
    status.set_visible(false);

    vbox.append(&button);
    vbox.append(&status);
    window.set_child(Some(&vbox));

    // State
    let state = Rc::new(RefCell::new(State::Idle));
    let recorder = Rc::new(RefCell::new(Recorder::new().expect("Failed to init audio")));
    // Flag: text is on clipboard, waiting for user to click target window
    let pending_paste = Rc::new(RefCell::new(false));

    // Open DB
    let db = Arc::new(Mutex::new(
        Db::open(&config.db_path).expect("Failed to open database"),
    ));

    // Click handler
    let btn = button.clone();
    let st = status.clone();
    let state_c = Rc::clone(&state);
    let rec_c = Rc::clone(&recorder);
    let config_c = Arc::clone(&config);
    let db_c = Arc::clone(&db);
    let pp = Rc::clone(&pending_paste);

    button.connect_clicked(move |_| {
        let current = *state_c.borrow();
        match current {
            State::Idle => {
                // Start recording
                if let Err(e) = rec_c.borrow_mut().start() {
                    eprintln!("Record start error: {e}");
                    st.set_label(&format!("Err: {e}"));
                    st.set_visible(true);
                    return;
                }
                *state_c.borrow_mut() = State::Recording;
                btn.add_css_class("recording");
                st.set_label("Recording...");
                st.set_visible(true);
            }
            State::Recording => {
                // Stop recording
                *state_c.borrow_mut() = State::Processing;
                btn.remove_css_class("recording");
                btn.set_sensitive(false);
                st.set_label("Transcribing...");

                let wav = match rec_c.borrow_mut().stop() {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("Record stop error: {e}");
                        st.set_label(&format!("Err: {e}"));
                        *state_c.borrow_mut() = State::Idle;
                        btn.set_sensitive(true);
                        return;
                    }
                };

                let api_key = config_c.groq_api_key.clone();
                let model = config_c.groq_model.clone();
                let db_inner = Arc::clone(&db_c);

                // Channel to receive result on GTK thread
                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

                // Spawn background thread with tokio for the API call
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(crate::api::transcribe(&api_key, &model, wav));
                    let _ = tx.send(result);
                });

                // Poll for result on GTK main thread
                let btn2 = btn.clone();
                let st2 = st.clone();
                let state_c2 = Rc::clone(&state_c);
                let pp2 = Rc::clone(&pp);
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    match rx.try_recv() {
                        Ok(Ok(text)) => {
                            // Save to DB
                            if let Ok(db) = db_inner.lock() {
                                if let Err(e) = db.insert(&text) {
                                    eprintln!("DB insert error: {e}");
                                }
                            }
                            // Put text on clipboard
                            match crate::input::copy_to_clipboard(&text) {
                                Ok(_) => {
                                    eprintln!("Clipboard ready, waiting for focus change...");
                                    *pp2.borrow_mut() = true;
                                    st2.set_label("Click target \u{2192}");
                                }
                                Err(e) => {
                                    eprintln!("Clipboard error: {e}");
                                    st2.set_label("Error!");
                                }
                            }
                            *state_c2.borrow_mut() = State::Idle;
                            btn2.set_sensitive(true);
                            glib::ControlFlow::Break
                        }
                        Ok(Err(e)) => {
                            eprintln!("Transcription error: {e}");
                            st2.set_label("Error!");
                            let st3 = st2.clone();
                            glib::timeout_add_local_once(
                                std::time::Duration::from_secs(3),
                                move || st3.set_visible(false),
                            );
                            *state_c2.borrow_mut() = State::Idle;
                            btn2.set_sensitive(true);
                            glib::ControlFlow::Break
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            glib::ControlFlow::Continue
                        }
                        Err(_) => {
                            // Sender dropped
                            *state_c2.borrow_mut() = State::Idle;
                            btn2.set_sensitive(true);
                            glib::ControlFlow::Break
                        }
                    }
                });
            }
            State::Processing => {
                // Ignore clicks while processing
            }
        }
    });

    // When LinWhisper loses focus and a paste is pending, fire Ctrl+V
    let pp_focus = Rc::clone(&pending_paste);
    let st_focus = status.clone();
    window.connect_is_active_notify(move |_win| {
        // is-active just changed; if it's now false, window lost focus
        if !_win.is_active() && *pp_focus.borrow() {
            *pp_focus.borrow_mut() = false;
            eprintln!("Focus lost — pasting...");
            // Paste on a short delay so the target window is fully focused
            let st_f = st_focus.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                if let Err(e) = crate::input::simulate_paste() {
                    eprintln!("Paste error: {e}");
                }
                st_f.set_label("Pasted!");
                let st_f2 = st_f.clone();
                glib::timeout_add_local_once(
                    std::time::Duration::from_secs(2),
                    move || st_f2.set_visible(false),
                );
            });
        }
    });

    // Right-click menu
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3); // right-click
    let db_menu = Arc::clone(&db);
    let win_ref = window.clone();
    gesture.connect_released(move |_, _, _, _| {
        show_context_menu(&win_ref, &db_menu);
    });
    window.add_controller(gesture);

    // Position: bottom-right, always-on-top
    window.connect_realize(|win| {
        if let Some(surface) = win.surface() {
            if let Some(toplevel) = surface.downcast_ref::<gdk::Toplevel>() {
                toplevel.set_decorated(false);
            }
        }
        let w = win.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
            position_bottom_right(&w);
        });
    });

    window.present();
}

fn position_bottom_right(_window: &gtk4::ApplicationWindow) {
    if let Some(display) = gdk::Display::default() {
        let monitors = display.monitors();
        if let Some(monitor) = monitors.item(0).and_then(|m| m.downcast::<gdk::Monitor>().ok()) {
            let geom = monitor.geometry();
            let x = geom.x() + geom.width() - 80;
            let y = geom.y() + geom.height() - 120;

            let title = "LinWhisper";
            // Position via xdotool (X11)
            let _ = std::process::Command::new("xdotool")
                .args([
                    "search", "--name", title,
                    "windowmove", &x.to_string(), &y.to_string(),
                ])
                .status();

            // Always-on-top via wmctrl or xdotool
            let _ = std::process::Command::new("xdotool")
                .args(["search", "--name", title, "windowactivate", "--sync"])
                .status();
            let _ = std::process::Command::new("wmctrl")
                .args(["-r", title, "-b", "add,above"])
                .status();
        }
    }
}

fn show_context_menu(window: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let dialog = gtk4::Window::builder()
        .title("LinWhisper History")
        .default_width(400)
        .default_height(300)
        .transient_for(window)
        .modal(true)
        .build();

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    let header = gtk4::Label::new(Some("Recent Transcriptions"));
    header.add_css_class("heading");
    vbox.append(&header);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);

    let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    if let Ok(db) = db.lock() {
        if let Ok(entries) = db.recent(20) {
            if entries.is_empty() {
                let empty = gtk4::Label::new(Some("No transcriptions yet."));
                list_box.append(&empty);
            } else {
                for entry in entries {
                    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    let time = gtk4::Label::new(Some(&entry.created_at));
                    time.set_halign(gtk4::Align::Start);
                    time.set_opacity(0.6);

                    let text = gtk4::Label::new(Some(&entry.text));
                    text.set_halign(gtk4::Align::Start);
                    text.set_wrap(true);
                    text.set_selectable(true);

                    row.append(&time);
                    row.append(&text);

                    let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                    list_box.append(&row);
                    list_box.append(&sep);
                }
            }
        }
    }

    scroll.set_child(Some(&list_box));
    vbox.append(&scroll);

    // Quit button
    let quit_btn = gtk4::Button::with_label("Quit LinWhisper");
    quit_btn.connect_clicked(move |_| {
        std::process::exit(0);
    });
    vbox.append(&quit_btn);

    dialog.set_child(Some(&vbox));
    dialog.present();
}
