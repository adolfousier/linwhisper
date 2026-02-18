use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::audio::Recorder;
use crate::config::{Config, TranscriptionService};
use crate::db::Db;
use crate::local_stt::LocalWhisper;

const ICON_MIC: &str = "audio-input-microphone-symbolic";
const NOTIFICATION_SOUND: &[u8] = include_bytes!("audio/notification.wav");

fn play_notification() {
    std::thread::spawn(|| {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;
        if let Ok((_stream, handle)) = OutputStream::try_default()
            && let Ok(sink) = Sink::try_new(&handle)
            && let Ok(source) = Decoder::new(Cursor::new(NOTIFICATION_SOUND))
        {
            sink.append(source);
            sink.sleep_until_end();
        }
    });
}

const CSS: &str = r#"
    window {
        background-color: transparent;
    }
    .mic-btn {
        min-width: 72px;
        min-height: 72px;
        border-radius: 9999px;
        background-image: none;
        background-color: #dc2626;
        color: white;
        font-size: 32px;
        font-weight: 600;
        border: none;
        box-shadow: none;
        outline: none;
        -gtk-icon-shadow: none;
        -gtk-icon-size: 32px;
        padding: 0;
    }
    .mic-btn:hover {
        background-image: none;
        background-color: #b91c1c;
        box-shadow: none;
    }
    .mic-btn:active {
        background-image: none;
        background-color: #991b1b;
        box-shadow: none;
    }
    .mic-btn.recording,
    .mic-btn.recording:hover {
        background-image: none;
        background-color: #16a34a;
        box-shadow: none;
        animation: pulse 1s ease-in-out infinite;
    }
    .mic-btn.processing,
    .mic-btn.processing:hover {
        background-image: none;
        background-color: #d97706;
        box-shadow: none;
    }
    .mic-btn.done,
    .mic-btn.done:hover {
        background-image: none;
        background-color: #16a34a;
        box-shadow: none;
    }
    @keyframes pulse {
        0%   { opacity: 1.0; }
        50%  { opacity: 0.7; }
        100% { opacity: 1.0; }
    }
    .status-label {
        color: #e2e8f0;
        font-size: 12px;
        font-weight: 500;
        background-color: rgba(15, 23, 42, 0.75);
        border-radius: 6px;
        padding: 3px 8px;
    }
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Idle,
    Recording,
    Processing,
}

struct RuntimeState {
    active_service: TranscriptionService,
    local_whisper: Option<Arc<LocalWhisper>>,
    downloading: bool,
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
        .title("WhisperCrabs")
        .default_width(88)
        .default_height(100)
        .decorated(false)
        .resizable(false)
        .build();

    // Layout
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vbox.set_halign(gtk4::Align::Center);
    vbox.set_valign(gtk4::Align::Center);

    // The mic button (no keyboard activation to prevent accidental recordings)
    let icon = gtk4::Image::from_icon_name(ICON_MIC);
    icon.set_pixel_size(32);
    let button = gtk4::Button::new();
    button.set_child(Some(&icon));
    button.add_css_class("mic-btn");
    button.set_size_request(72, 72);
    button.set_halign(gtk4::Align::Center);
    button.set_focusable(false);

    let status = gtk4::Label::new(Some(" "));
    status.add_css_class("status-label");
    status.set_opacity(0.0);

    vbox.append(&button);
    vbox.append(&status);

    // WindowHandle wraps everything — makes the empty area around
    // the button draggable like a titlebar. Clicks on the Button
    // itself still go through to the button's click handler.
    let handle = gtk4::WindowHandle::new();
    handle.set_child(Some(&vbox));
    window.set_child(Some(&handle));

    // Open DB
    let db = Arc::new(Mutex::new(
        Db::open(&config.db_path).expect("Failed to open database"),
    ));

    // Determine initial mode: DB setting overrides env var
    let initial_service = {
        let db_mode = db
            .lock()
            .ok()
            .and_then(|d| d.get_setting("transcription_mode").ok().flatten());
        match db_mode.as_deref() {
            Some("local") => TranscriptionService::Local,
            Some("api") => TranscriptionService::Api,
            None => config.transcription_service,
            _ => config.transcription_service,
        }
    };

    // Init local whisper only if Local mode AND model file exists
    let initial_whisper: Option<Arc<LocalWhisper>> =
        if initial_service == TranscriptionService::Local && config.whisper_model_path.exists() {
            match LocalWhisper::new(&config.whisper_model_path) {
                Ok(w) => Some(Arc::new(w)),
                Err(e) => {
                    eprintln!("Failed to load whisper model: {e}");
                    None
                }
            }
        } else {
            None
        };

    // Runtime state (UI-thread only)
    let runtime = Rc::new(RefCell::new(RuntimeState {
        active_service: initial_service,
        local_whisper: initial_whisper,
        downloading: false,
    }));

    // Shared state
    let state = Rc::new(RefCell::new(State::Idle));
    let recorder = Rc::new(RefCell::new(Recorder::new().expect("Failed to init audio")));

    // --- Left-click handler (on the Button) ---
    let btn = button.clone();
    let st = status.clone();
    let state_c = Rc::clone(&state);
    let rec_c = Rc::clone(&recorder);
    let config_c = Arc::clone(&config);
    let db_c = Arc::clone(&db);
    let runtime_c = Rc::clone(&runtime);

    button.connect_clicked(move |_| {
        let current = *state_c.borrow();
        match current {
            State::Idle => {
                // Guard: block recording during model download
                if runtime_c.borrow().downloading {
                    st.set_label("Downloading model...");
                    st.set_opacity(1.0);
                    return;
                }

                // Guard: Local mode without loaded model
                let rt = runtime_c.borrow();
                if rt.active_service == TranscriptionService::Local && rt.local_whisper.is_none() {
                    drop(rt);
                    st.set_label("No local model loaded");
                    st.set_opacity(1.0);
                    return;
                }

                // Guard: API mode without API key
                if rt.active_service == TranscriptionService::Api && config_c.api_key.is_none() {
                    drop(rt);
                    st.set_label("No API key set");
                    st.set_opacity(1.0);
                    return;
                }
                drop(rt);

                if let Err(e) = rec_c.borrow_mut().start() {
                    eprintln!("Record start error: {e}");
                    st.set_label(&format!("Err: {e}"));
                    st.set_opacity(1.0);
                    return;
                }
                *state_c.borrow_mut() = State::Recording;
                btn.add_css_class("recording");
                btn.remove_css_class("done");

                st.set_label("Recording...");
                st.set_opacity(1.0);
            }
            State::Recording => {
                *state_c.borrow_mut() = State::Processing;
                btn.remove_css_class("recording");
                btn.add_css_class("processing");

                st.set_label("Transcribing...");

                let wav = match rec_c.borrow_mut().stop() {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("Record stop error: {e}");
                        st.set_label(&format!("Err: {e}"));
                        *state_c.borrow_mut() = State::Idle;
                        btn.remove_css_class("processing");
                        return;
                    }
                };

                let db_inner = Arc::clone(&db_c);
                let sample_rate = rec_c.borrow().sample_rate();

                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

                let rt = runtime_c.borrow();
                match rt.active_service {
                    TranscriptionService::Api => {
                        let base_url = config_c.api_base_url.clone();
                        let api_key = config_c.api_key.clone().unwrap();
                        let model = config_c.api_model.clone();
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let result = rt.block_on(crate::api::transcribe(
                                &base_url, &api_key, &model, wav,
                            ));
                            let _ = tx.send(result);
                        });
                    }
                    TranscriptionService::Local => {
                        let whisper = rt.local_whisper.clone().unwrap();
                        std::thread::spawn(move || {
                            let result = whisper.transcribe(&wav, sample_rate);
                            let _ = tx.send(result);
                        });
                    }
                }
                drop(rt);

                let btn2 = btn.clone();
                let st2 = st.clone();
                let state_c2 = Rc::clone(&state_c);
                let notify = config_c.sound_notification;
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    match rx.try_recv() {
                        Ok(Ok(text)) => {
                            if let Ok(db) = db_inner.lock()
                                && let Err(e) = db.insert(&text)
                            {
                                eprintln!("DB insert error: {e}");
                            }
                            match crate::input::copy_to_clipboard(&text) {
                                Ok(_) => {
                                    if notify {
                                        play_notification();
                                    }
                                    btn2.remove_css_class("processing");
                                    btn2.add_css_class("done");

                                    st2.set_label("Copied!");
                                    let st3 = st2.clone();
                                    let btn3 = btn2.clone();
                                    glib::timeout_add_local_once(
                                        std::time::Duration::from_secs(2),
                                        move || {
                                            st3.set_opacity(0.0);
                                            btn3.remove_css_class("done");

                                        },
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Clipboard error: {e}");
                                    btn2.remove_css_class("processing");

                                    st2.set_label("Error!");
                                    let st3 = st2.clone();
                                    glib::timeout_add_local_once(
                                        std::time::Duration::from_secs(3),
                                        move || st3.set_opacity(0.0),
                                    );
                                }
                            }
                            *state_c2.borrow_mut() = State::Idle;
                            glib::ControlFlow::Break
                        }
                        Ok(Err(e)) => {
                            eprintln!("Transcription error: {e}");
                            btn2.remove_css_class("processing");
                            st2.set_label("Error!");
                            let st3 = st2.clone();
                            glib::timeout_add_local_once(
                                std::time::Duration::from_secs(3),
                                move || st3.set_opacity(0.0),
                            );
                            *state_c2.borrow_mut() = State::Idle;
                            glib::ControlFlow::Break
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                        Err(_) => {
                            *state_c2.borrow_mut() = State::Idle;
                            btn2.remove_css_class("processing");
                            glib::ControlFlow::Break
                        }
                    }
                });
            }
            State::Processing => {}
        }
    });

    // --- Right-click popover menu (on the button) ---
    let mode_action = gtk4::gio::SimpleAction::new_stateful(
        "transcription-mode",
        Some(&String::static_variant_type()),
        &(if initial_service == TranscriptionService::Api {
            "api"
        } else {
            "local"
        })
        .to_variant(),
    );

    let transcription_section = gtk4::gio::Menu::new();
    transcription_section.append(Some("API Mode"), Some("app.transcription-mode::api"));
    transcription_section.append(Some("Local Mode"), Some("app.transcription-mode::local"));

    let actions_section = gtk4::gio::Menu::new();
    actions_section.append(Some("History"), Some("app.show-history"));
    actions_section.append(Some("Quit"), Some("app.quit"));

    let menu = gtk4::gio::Menu::new();
    menu.append_section(Some("Transcription"), &transcription_section);
    menu.append_section(None, &actions_section);

    let popover = gtk4::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(&button);
    popover.set_has_arrow(true);

    // Right-click on button → show our popover, suppress WM menu
    let pop = popover.clone();
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    gesture.connect_pressed(move |g, _, _, _| {
        g.set_state(gtk4::EventSequenceState::Claimed);
        pop.popup();
    });
    button.add_controller(gesture);

    // Action: transcription mode switch
    let runtime_mode = Rc::clone(&runtime);
    let state_mode = Rc::clone(&state);
    let config_mode = Arc::clone(&config);
    let db_mode = Arc::clone(&db);
    let status_mode = status.clone();
    mode_action.connect_activate(move |action, param| {
        let chosen: String = param.unwrap().get::<String>().unwrap();

        // Guard: block mode switch during recording/processing
        if *state_mode.borrow() != State::Idle {
            return;
        }

        // Guard: block mode switch during download
        if runtime_mode.borrow().downloading {
            return;
        }

        let current = if runtime_mode.borrow().active_service == TranscriptionService::Api {
            "api"
        } else {
            "local"
        };
        if chosen == current {
            return;
        }

        if chosen == "api" {
            switch_to_api(
                &runtime_mode,
                &config_mode,
                &db_mode,
                action,
                &status_mode,
            );
        } else {
            switch_to_local(
                &runtime_mode,
                &config_mode,
                &db_mode,
                action,
                &status_mode,
            );
        }
    });
    app.add_action(&mode_action);

    // Action: show history
    let history_action = gtk4::gio::SimpleAction::new("show-history", None);
    let db_hist = Arc::clone(&db);
    let win_ref = window.clone();
    history_action.connect_activate(move |_, _| {
        show_history_dialog(&win_ref, &db_hist);
    });
    app.add_action(&history_action);

    // Action: quit
    let quit_action = gtk4::gio::SimpleAction::new("quit", None);
    quit_action.connect_activate(move |_, _| {
        std::process::exit(0);
    });
    app.add_action(&quit_action);

    // --- Save position on close ---
    let db_close = Arc::clone(&db);
    window.connect_close_request(move |win| {
        save_window_position(win, &db_close);
        glib::Propagation::Proceed
    });

    // --- Position: saved or bottom-right ---
    let db_pos = Arc::clone(&db);
    window.connect_realize(move |win| {
        if let Some(surface) = win.surface()
            && let Some(toplevel) = surface.downcast_ref::<gdk::Toplevel>()
        {
            toplevel.set_decorated(false);
        }
        let w = win.clone();
        let db_p = Arc::clone(&db_pos);
        glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
            position_window(&w, &db_p);
        });
    });

    // --- Esc key: stop recording ---
    let esc_btn = button.clone();
    let esc_state = Rc::clone(&state);
    let esc_shortcut = gtk4::Shortcut::new(
        gtk4::ShortcutTrigger::parse_string("Escape"),
        Some(gtk4::CallbackAction::new(move |_, _| {
            if *esc_state.borrow() == State::Recording {
                esc_btn.emit_clicked();
            }
            glib::Propagation::Stop
        })),
    );
    let esc_controller = gtk4::ShortcutController::new();
    esc_controller.set_scope(gtk4::ShortcutScope::Global);
    esc_controller.add_shortcut(esc_shortcut);
    window.add_controller(esc_controller);

    // --- D-Bus action: "record" — triggered by GNOME shortcut ---
    let record_action = gtk4::gio::SimpleAction::new("record", None);
    let btn_rec = button.clone();
    let state_rec = Rc::clone(&state);
    let win_rec = window.clone();
    record_action.connect_activate(move |_, _| {
        eprintln!("[dbus] 'record' action activated");
        win_rec.present();
        // GNOME Wayland: force-activate via Shell D-Bus (falls back silently on other DEs)
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("gdbus")
                .args([
                    "call", "--session",
                    "--dest=org.gnome.Shell",
                    "--object-path=/org/gnome/Shell",
                    "--method=org.gnome.Shell.Eval",
                    r#"global.get_window_actors().find(a=>a.meta_window.title==='WhisperCrabs')?.meta_window.activate(0)"#,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        if *state_rec.borrow() == State::Idle {
            btn_rec.emit_clicked();
        }
    });
    app.add_action(&record_action);

    // --- D-Bus action: "stop" — triggered by GNOME shortcut ---
    let stop_action = gtk4::gio::SimpleAction::new("stop", None);
    let btn_stop = button.clone();
    let state_stop = Rc::clone(&state);
    stop_action.connect_activate(move |_, _| {
        eprintln!("[dbus] 'stop' action activated");
        if *state_stop.borrow() == State::Recording {
            btn_stop.emit_clicked();
        }
    });
    app.add_action(&stop_action);

    window.present();
}

fn switch_to_api(
    runtime: &Rc<RefCell<RuntimeState>>,
    config: &Arc<Config>,
    db: &Arc<Mutex<Db>>,
    action: &gtk4::gio::SimpleAction,
    status: &gtk4::Label,
) {
    let mut rt = runtime.borrow_mut();
    rt.active_service = TranscriptionService::Api;
    rt.local_whisper = None;
    drop(rt);

    // Delete model file to free disk space
    if config.whisper_model_path.exists()
        && let Err(e) = std::fs::remove_file(&config.whisper_model_path)
    {
        eprintln!("Failed to delete model file: {e}");
    }

    // Persist to DB
    if let Ok(d) = db.lock() {
        let _ = d.set_setting("transcription_mode", "api");
    }

    action.set_state(&"api".to_variant());

    status.set_label("API mode");
    status.set_opacity(1.0);
    let st = status.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
        st.set_opacity(0.0);
    });
}

fn switch_to_local(
    runtime: &Rc<RefCell<RuntimeState>>,
    config: &Arc<Config>,
    db: &Arc<Mutex<Db>>,
    action: &gtk4::gio::SimpleAction,
    status: &gtk4::Label,
) {
    // Set active service immediately so the menu reflects the choice
    runtime.borrow_mut().active_service = TranscriptionService::Local;
    action.set_state(&"local".to_variant());

    // Persist to DB
    if let Ok(d) = db.lock() {
        let _ = d.set_setting("transcription_mode", "local");
    }

    if config.whisper_model_path.exists() {
        // Model exists — load it on a background thread
        load_whisper_model(runtime, config, action, status);
    } else {
        // Model missing — download then load
        download_and_load_model(runtime, config, action, status);
    }
}

fn load_whisper_model(
    runtime: &Rc<RefCell<RuntimeState>>,
    config: &Arc<Config>,
    action: &gtk4::gio::SimpleAction,
    status: &gtk4::Label,
) {
    status.set_label("Loading model...");
    status.set_opacity(1.0);

    let model_path = config.whisper_model_path.clone();
    let (tx, rx) = std::sync::mpsc::channel::<Result<Arc<LocalWhisper>, String>>();

    std::thread::spawn(move || {
        let result = LocalWhisper::new(&model_path).map(Arc::new);
        let _ = tx.send(result);
    });

    let runtime_c = Rc::clone(runtime);
    let action_c = action.clone();
    let st = status.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match rx.try_recv() {
            Ok(Ok(whisper)) => {
                runtime_c.borrow_mut().local_whisper = Some(whisper);
                st.set_label("Local mode ready");
                let st2 = st.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    st2.set_opacity(0.0);
                });
                glib::ControlFlow::Break
            }
            Ok(Err(e)) => {
                eprintln!("Failed to load whisper model: {e}");
                // Revert to API
                runtime_c.borrow_mut().active_service = TranscriptionService::Api;
                action_c.set_state(&"api".to_variant());
                st.set_label("Model load failed");
                let st2 = st.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                    st2.set_opacity(0.0);
                });
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => {
                runtime_c.borrow_mut().active_service = TranscriptionService::Api;
                action_c.set_state(&"api".to_variant());
                st.set_label("Model load failed");
                let st2 = st.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                    st2.set_opacity(0.0);
                });
                glib::ControlFlow::Break
            }
        }
    });
}

/// Download progress messages sent from the background thread
enum DownloadMsg {
    Progress(u64, Option<u64>), // downloaded, total
    Done,
    Error(String),
}

fn download_and_load_model(
    runtime: &Rc<RefCell<RuntimeState>>,
    config: &Arc<Config>,
    action: &gtk4::gio::SimpleAction,
    status: &gtk4::Label,
) {
    runtime.borrow_mut().downloading = true;

    status.set_label("Downloading model...");
    status.set_opacity(1.0);

    let url = config.whisper_model_url();
    let model_path = config.whisper_model_path.clone();
    let part_path = model_path.with_extension("bin.part");

    let (tx, rx) = std::sync::mpsc::channel::<DownloadMsg>();

    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let resp = reqwest::blocking::Client::new()
                .get(&url)
                .send()
                .map_err(|e| format!("Download request failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("Download failed: HTTP {}", resp.status()));
            }

            let total = resp.content_length();
            let mut downloaded: u64 = 0;

            let mut file = std::fs::File::create(&part_path)
                .map_err(|e| format!("Failed to create file: {e}"))?;

            use std::io::{Read, Write};
            let mut reader = resp;
            let mut buf = [0u8; 65536];
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|e| format!("Download read error: {e}"))?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n])
                    .map_err(|e| format!("File write error: {e}"))?;
                downloaded += n as u64;
                let _ = tx.send(DownloadMsg::Progress(downloaded, total));
            }

            // Rename .part → final path
            std::fs::rename(&part_path, &model_path)
                .map_err(|e| format!("Failed to rename model file: {e}"))?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                let _ = tx.send(DownloadMsg::Done);
            }
            Err(e) => {
                // Clean up partial file
                let _ = std::fs::remove_file(&part_path);
                let _ = tx.send(DownloadMsg::Error(e));
            }
        }
    });

    let runtime_c = Rc::clone(runtime);
    let config_c = Arc::clone(config);
    let action_c = action.clone();
    let st = status.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        // Drain all pending messages, keep the last one
        let mut last_msg = None;
        while let Ok(msg) = rx.try_recv() {
            last_msg = Some(msg);
        }

        match last_msg {
            Some(DownloadMsg::Progress(downloaded, total)) => {
                let dl_mb = downloaded as f64 / (1024.0 * 1024.0);
                if let Some(t) = total {
                    let total_mb = t as f64 / (1024.0 * 1024.0);
                    st.set_label(&format!("Downloading: {dl_mb:.0} / {total_mb:.0} MB"));
                } else {
                    st.set_label(&format!("Downloading: {dl_mb:.0} MB"));
                }
                glib::ControlFlow::Continue
            }
            Some(DownloadMsg::Done) => {
                runtime_c.borrow_mut().downloading = false;
                st.set_label("Loading model...");
                // Now load the model
                load_whisper_model(&runtime_c, &config_c, &action_c, &st);
                glib::ControlFlow::Break
            }
            Some(DownloadMsg::Error(e)) => {
                eprintln!("Model download failed: {e}");
                {
                    let mut rt = runtime_c.borrow_mut();
                    rt.downloading = false;
                    rt.active_service = TranscriptionService::Api;
                }
                action_c.set_state(&"api".to_variant());
                st.set_label("Download failed");
                let st2 = st.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                    st2.set_opacity(0.0);
                });
                glib::ControlFlow::Break
            }
            None => glib::ControlFlow::Continue,
        }
    });
}


fn save_window_position(win: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    #[cfg(not(target_os = "linux"))]
    let _ = (&win, &db);

    #[cfg(target_os = "linux")]
    {
        let title = win.title().map(|t| t.to_string()).unwrap_or_default();
        if let Ok(output) = std::process::Command::new("xdotool")
            .args(["search", "--name", &title, "getwindowgeometry"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(pos) = line.strip_prefix("  Position: ")
                    && let Some((xs, ys)) = pos.split_once(',')
                {
                    let x = xs.trim();
                    let y = ys.split_whitespace().next().unwrap_or("0");
                    if let Ok(db) = db.lock() {
                        let _ = db.set_setting("window_x", x);
                        let _ = db.set_setting("window_y", y);
                    }
                }
            }
        }
    }
}

fn position_window(_window: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let saved = db.lock().ok().and_then(|db| {
        let x = db.get_setting("window_x").ok()??.parse::<i32>().ok()?;
        let y = db.get_setting("window_y").ok()??.parse::<i32>().ok()?;
        Some((x, y))
    });

    let (x, y) = match saved {
        Some(pos) => pos,
        None => {
            if let Some(display) = gdk::Display::default() {
                let monitors = display.monitors();
                if let Some(monitor) =
                    monitors.item(0).and_then(|m| m.downcast::<gdk::Monitor>().ok())
                {
                    let geom = monitor.geometry();
                    (
                        geom.x() + geom.width() - 100,
                        geom.y() + geom.height() - 140,
                    )
                } else {
                    (100, 100)
                }
            } else {
                (100, 100)
            }
        }
    };

    #[cfg(target_os = "linux")]
    {
        let title = "WhisperCrabs";
        let _ = std::process::Command::new("xdotool")
            .args([
                "search", "--name", title,
                "windowmove", &x.to_string(), &y.to_string(),
            ])
            .status();
    }
}

fn show_history_dialog(_window: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let dialog = gtk4::Window::builder()
        .title("WhisperCrabs History")
        .default_width(400)
        .default_height(300)
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

    if let Ok(db) = db.lock()
        && let Ok(entries) = db.recent(20)
    {
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

    scroll.set_child(Some(&list_box));
    vbox.append(&scroll);

    dialog.set_child(Some(&vbox));
    dialog.present();
}
