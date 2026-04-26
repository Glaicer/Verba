use std::{cell::Cell, rc::Rc};

use gtk4::{
    gio,
    glib::{self, Propagation},
    prelude::*,
    Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, Entry, Label, Orientation,
    ScrolledWindow, Separator, TextBuffer, TextView, WrapMode,
};

use crate::{
    app_runtime::{AppRuntime, AppState},
    config::{AppConfig, ConfigStore, Preset},
    gui::actions::{accelerators_for_action, GuiAction},
    gui::settings_dialog::SettingsDialog,
    secrets::SecretStore,
};

const DEFAULT_LANGUAGES: [&str; 10] = [
    "English",
    "Russian",
    "German",
    "French",
    "Spanish",
    "Chinese",
    "Japanese",
    "Korean",
    "Italian",
    "Portuguese",
];

#[derive(Clone, Debug)]
pub struct MainWindowController {
    window: ApplicationWindow,
}

impl MainWindowController {
    pub fn build<S>(
        app: &Application,
        config: AppConfig,
        store: ConfigStore,
        secrets: S,
        runtime: AppRuntime,
    ) -> Self
    where
        S: SecretStore + Clone + 'static,
    {
        install_accelerators(app);

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Verba")
            .default_width(config.ui.window_width)
            .default_height(config.ui.window_height)
            .build();

        let language_entry = Entry::builder()
            .hexpand(true)
            .text(&config.ui.last_language)
            .build();
        language_entry.set_placeholder_text(Some("English"));

        let preset_combo = build_preset_dropdown(&config.presets, selected_preset_index(&config));

        let input_buffer = TextBuffer::new(None);
        let input_view = TextView::builder()
            .buffer(&input_buffer)
            .wrap_mode(WrapMode::WordChar)
            .hexpand(true)
            .vexpand(true)
            .build();

        let output_buffer = TextBuffer::new(None);
        let output_view = TextView::builder()
            .buffer(&output_buffer)
            .wrap_mode(WrapMode::WordChar)
            .editable(false)
            .cursor_visible(false)
            .hexpand(true)
            .vexpand(true)
            .build();

        let status_label = Label::new(None);
        status_label.set_xalign(0.0);

        let copy_button = Button::with_label("Copy");
        copy_button.set_sensitive(false);

        let settings_button = Button::with_label("Settings");
        let close_button = Button::with_label("Close");
        let translate_button = Button::with_label("Translate");

        let layout = build_layout(
            &language_entry,
            &preset_combo,
            &input_view,
            &output_view,
            &copy_button,
            &settings_button,
            &close_button,
            &translate_button,
            &status_label,
        );
        window.set_child(Some(&layout));

        wire_close_to_hide(&window, runtime.clone());
        wire_buttons(
            &window,
            &language_entry,
            &preset_combo,
            &input_buffer,
            &output_buffer,
            &copy_button,
            &settings_button,
            &close_button,
            &translate_button,
            &status_label,
            config,
            store,
            secrets,
            runtime,
        );

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn attach_runtime_sync<S>(
        &self,
        config: AppConfig,
        store: ConfigStore,
        secrets: S,
        runtime: AppRuntime,
    ) where
        S: SecretStore + Clone + 'static,
    {
        let window = self.window.clone();
        let last_state = Rc::new(Cell::new(runtime.state()));
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let state = runtime.state();
            if state != last_state.get() {
                match state {
                    AppState::Hidden | AppState::HiddenTranslating => window.hide(),
                    AppState::VisibleIdle | AppState::VisibleTranslating => window.present(),
                    AppState::SettingsOpen => {
                        window.present();
                        let parent = window.clone();
                        let store = store.clone();
                        let secrets = secrets.clone();
                        let config = config.clone();
                        glib::MainContext::default().spawn_local(async move {
                            let dialog =
                                SettingsDialog::build(&parent, store, secrets, config).await;
                            dialog.present();
                        });
                    }
                    AppState::Exiting | AppState::CancellingThenExit => {}
                }
                last_state.set(state);
            }

            if runtime.is_exiting() {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}

pub fn default_languages() -> &'static [&'static str] {
    &DEFAULT_LANGUAGES
}

pub fn selected_preset_index(config: &AppConfig) -> Option<u32> {
    if config.presets.is_empty() {
        return None;
    }

    let index = config
        .presets
        .iter()
        .position(|preset| preset.id == config.ui.last_preset_id)
        .unwrap_or(0);
    Some(index as u32)
}

fn install_accelerators(app: &Application) {
    for action in [
        GuiAction::Translate,
        GuiAction::Close,
        GuiAction::FocusLanguage,
        GuiAction::FocusPreset,
        GuiAction::CopyResult,
    ] {
        app.set_accels_for_action(
            &action.detailed_action_name(),
            accelerators_for_action(action),
        );
    }
}

fn build_preset_dropdown(presets: &[Preset], active: Option<u32>) -> ComboBoxText {
    let combo = ComboBoxText::new();
    for preset in presets {
        combo.append(Some(&preset.id), &preset.name);
    }
    combo.set_active(active);
    combo
}

#[allow(clippy::too_many_arguments)]
fn build_layout(
    language_entry: &Entry,
    preset_combo: &ComboBoxText,
    input_view: &TextView,
    output_view: &TextView,
    copy_button: &Button,
    settings_button: &Button,
    close_button: &Button,
    translate_button: &Button,
    status_label: &Label,
) -> GtkBox {
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let top_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    top_bar.append(&Label::new(Some("Translate to")));
    top_bar.append(language_entry);
    top_bar.append(&Label::new(Some("Preset")));
    top_bar.append(preset_combo);

    let text_pane = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();
    text_pane.append(&scrolled(input_view));

    let output_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .vexpand(true)
        .build();
    let copy_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .halign(gtk4::Align::End)
        .build();
    copy_bar.append(copy_button);
    output_box.append(&copy_bar);
    output_box.append(&scrolled(output_view));
    text_pane.append(&output_box);

    let bottom_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    bottom_bar.append(settings_button);
    status_label.set_hexpand(true);
    bottom_bar.append(status_label);
    bottom_bar.append(close_button);
    bottom_bar.append(translate_button);

    root.append(&top_bar);
    root.append(&text_pane);
    root.append(&Separator::new(Orientation::Horizontal));
    root.append(&bottom_bar);
    root
}

fn scrolled(view: &TextView) -> ScrolledWindow {
    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_width(320)
        .min_content_height(280)
        .build();
    scroll.set_child(Some(view));
    scroll
}

fn wire_close_to_hide(window: &ApplicationWindow, runtime: AppRuntime) {
    let runtime_for_close = runtime.clone();
    window.connect_close_request(move |window| {
        runtime_for_close.hide_main_window();
        window.hide();
        Propagation::Stop
    });
}

#[allow(clippy::too_many_arguments)]
fn wire_buttons(
    window: &ApplicationWindow,
    language_entry: &Entry,
    preset_combo: &ComboBoxText,
    input_buffer: &TextBuffer,
    output_buffer: &TextBuffer,
    copy_button: &Button,
    settings_button: &Button,
    close_button: &Button,
    translate_button: &Button,
    status_label: &Label,
    config: AppConfig,
    store: ConfigStore,
    secrets: impl SecretStore + Clone + 'static,
    runtime: AppRuntime,
) {
    let window_for_close = window.clone();
    let runtime_for_close = runtime.clone();
    close_button.connect_clicked(move |_| {
        runtime_for_close.hide_main_window();
        window_for_close.hide();
    });

    let runtime_for_settings = runtime.clone();
    let parent_for_settings = window.clone();
    let config_for_settings = config.clone();
    let store_for_settings = store.clone();
    let secrets_for_settings = secrets.clone();
    settings_button.connect_clicked(move |_| {
        runtime_for_settings.open_settings();
        let parent = parent_for_settings.clone();
        let config = config_for_settings.clone();
        let store = store_for_settings.clone();
        let secrets = secrets_for_settings.clone();
        glib::MainContext::default().spawn_local(async move {
            let dialog = SettingsDialog::build(&parent, store, secrets, config).await;
            dialog.present();
        });
    });

    let output_for_copy = output_buffer.clone();
    let copy_button_for_copy = copy_button.clone();
    let copy_result = move || {
        let text = output_for_copy.text(
            &output_for_copy.start_iter(),
            &output_for_copy.end_iter(),
            false,
        );
        if text.is_empty() {
            return;
        }
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&text);
            copy_button_for_copy.set_label("Copied");
            let button = copy_button_for_copy.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                button.set_label("Copy");
            });
        }
    };
    let copy_result_button = copy_result.clone();
    copy_button.connect_clicked(move |_| copy_result_button());

    let runtime_for_translate = runtime.clone();
    let input_for_translate = input_buffer.clone();
    let language_for_translate = language_entry.clone();
    let preset_for_translate = preset_combo.clone();
    let translate_button_for_translate = translate_button.clone();
    let copy_button_for_translate = copy_button.clone();
    let status_for_translate = status_label.clone();
    translate_button.connect_clicked(move |_| {
        let input = input_for_translate.text(
            &input_for_translate.start_iter(),
            &input_for_translate.end_iter(),
            false,
        );
        if input.trim().is_empty() {
            status_for_translate.set_text("Enter text to translate.");
            return;
        }
        if language_for_translate.text().trim().is_empty() {
            status_for_translate.set_text("Enter target language.");
            return;
        }
        if preset_for_translate.active_id().is_none() {
            status_for_translate.set_text("Select a preset.");
            return;
        }

        runtime_for_translate.translate();
        translate_button_for_translate.set_sensitive(false);
        status_for_translate.set_text("Translating…");
        copy_button_for_translate.set_sensitive(false);
    });

    add_action(window, GuiAction::Translate, {
        let translate_button = translate_button.clone();
        move || translate_button.emit_clicked()
    });
    add_action(window, GuiAction::Close, {
        let close_button = close_button.clone();
        move || close_button.emit_clicked()
    });
    add_action(window, GuiAction::FocusLanguage, {
        let language_entry = language_entry.clone();
        move || {
            language_entry.grab_focus();
        }
    });
    add_action(window, GuiAction::FocusPreset, {
        let preset_combo = preset_combo.clone();
        move || {
            preset_combo.grab_focus();
        }
    });
    add_action(window, GuiAction::CopyResult, copy_result);
}

fn add_action<F>(window: &ApplicationWindow, action: GuiAction, callback: F)
where
    F: Fn() + 'static,
{
    let entry = gio::ActionEntry::builder(action.action_name())
        .activate(move |_, _, _| callback())
        .build();
    window.add_action_entries([entry]);
}
