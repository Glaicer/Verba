use std::{cell::Cell, rc::Rc, sync::mpsc};

use gtk4::{
    gio,
    glib::{self, Propagation},
    prelude::*,
    Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, Entry, Frame, Label,
    Orientation, ScrolledWindow, Separator, TextBuffer, TextView, WrapMode,
};

use crate::{
    app_runtime::{AppRuntime, AppState},
    config::{AppConfig, ConfigStore, Preset},
    gui::actions::{accelerators_for_action, GuiAction},
    gui::settings_dialog::SettingsDialog,
    notify::notify_send::NotifySend,
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
        S: SecretStore + Clone + Send + Sync + 'static,
    {
        install_accelerators(app);

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Verba")
            .default_width(config.ui.window_width)
            .default_height(config.ui.window_height)
            .build();

        let language_entry = ComboBoxText::builder()
            .has_entry(true)
            .hexpand(false)
            .build();
        language_entry.append(None, "English");
        language_entry.append(None, "Russian");
        if let Some(entry) = language_entry.child().and_downcast::<Entry>() {
            entry.set_width_chars(24);
            entry.set_placeholder_text(Some("English"));
            entry.set_text(&config.ui.last_language);
        }

        let preset_combo = build_preset_dropdown(&config.presets, selected_preset_index(&config));

        let input_buffer = TextBuffer::new(None);
        let input_view = TextView::builder()
            .buffer(&input_buffer)
            .wrap_mode(WrapMode::WordChar)
            .hexpand(true)
            .vexpand(true)
            .build();
        set_text_area_padding(&input_view);

        let output_buffer = TextBuffer::new(None);
        let output_view = TextView::builder()
            .buffer(&output_buffer)
            .wrap_mode(WrapMode::WordChar)
            .editable(false)
            .cursor_visible(false)
            .hexpand(true)
            .vexpand(true)
            .build();
        set_text_area_padding(&output_view);
        output_view.add_css_class("translation-empty");
        install_translation_empty_css();
        {
            let output_view_ref = output_view.clone();
            let output_buffer_ref = output_buffer.clone();
            output_buffer.connect_changed(move |_| {
                let text = output_buffer_ref.text(
                    &output_buffer_ref.start_iter(),
                    &output_buffer_ref.end_iter(),
                    false,
                );
                if text.is_empty() {
                    output_view_ref.add_css_class("translation-empty");
                } else {
                    output_view_ref.remove_css_class("translation-empty");
                }
            });
        }

        let status_label = Label::new(None);
        status_label.set_xalign(0.0);

        let input_clear_button = Button::with_label("Clear");
        input_clear_button.set_sensitive(false);
        let output_clear_button = Button::with_label("Clear");
        output_clear_button.set_sensitive(false);
        let copy_button = Button::with_label("Copy");
        copy_button.set_sensitive(false);

        {
            let btn = input_clear_button.clone();
            let buf = input_buffer.clone();
            input_buffer.connect_changed(move |_| {
                let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
                btn.set_sensitive(!text.is_empty());
            });
        }
        {
            let btn = output_clear_button.clone();
            let buf = output_buffer.clone();
            output_buffer.connect_changed(move |_| {
                let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
                btn.set_sensitive(!text.is_empty());
            });
        }

        let settings_button = Button::with_label("Settings");
        let close_button = Button::with_label("Close");
        let translate_button = Button::with_label("Translate");

        let layout = build_layout(
            &language_entry,
            &preset_combo,
            &input_view,
            &output_view,
            &input_clear_button,
            &output_clear_button,
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
            &input_clear_button,
            &output_clear_button,
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
        _config: AppConfig,
        store: ConfigStore,
        secrets: S,
        runtime: AppRuntime,
    ) where
        S: SecretStore + Clone + Send + Sync + 'static,
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
                        let config = runtime.config();
                        let runtime = runtime.clone();
                        glib::MainContext::default().spawn_local(async move {
                            let dialog =
                                SettingsDialog::build(&parent, store, secrets, config, runtime)
                                    .await;
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
    language_entry: &ComboBoxText,
    preset_combo: &ComboBoxText,
    input_view: &TextView,
    output_view: &TextView,
    input_clear_button: &Button,
    output_clear_button: &Button,
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

    let language_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    language_bar.append(&Label::new(Some("Translate to")));
    language_bar.append(language_entry);
    let preset_spacer = GtkBox::builder().width_request(24).build();
    language_bar.append(&preset_spacer);
    language_bar.append(&Label::new(Some("Preset")));
    language_bar.append(preset_combo);

    let control_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    let control_spacer = GtkBox::builder().hexpand(true).build();
    control_bar.append(input_clear_button);
    control_bar.append(&control_spacer);
    control_bar.append(output_clear_button);
    control_bar.append(copy_button);

    let text_pane = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();
    text_pane.append(&text_area_block("Original", input_view));
    text_pane.append(&text_area_block("Translation", output_view));

    let bottom_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    bottom_bar.append(settings_button);
    status_label.set_hexpand(true);
    bottom_bar.append(status_label);
    bottom_bar.append(close_button);
    bottom_bar.append(translate_button);

    root.append(&language_bar);
    root.append(&control_bar);
    root.append(&text_pane);
    root.append(&Separator::new(Orientation::Horizontal));
    root.append(&bottom_bar);
    root
}

fn text_area_block(label: &str, view: &TextView) -> GtkBox {
    let block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .vexpand(true)
        .build();

    let label = Label::new(Some(label));
    label.set_xalign(0.0);
    block.append(&label);

    let frame = Frame::builder().hexpand(true).vexpand(true).build();
    frame.set_child(Some(&scrolled(view)));
    block.append(&frame);
    block
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

fn set_text_area_padding(view: &TextView) {
    view.set_top_margin(8);
    view.set_bottom_margin(8);
    view.set_left_margin(8);
    view.set_right_margin(8);
}

fn install_translation_empty_css() {
    let provider = gtk4::CssProvider::new();
    if let Some(settings) = gtk4::Settings::default() {
        load_translation_empty_css(&provider, settings_prefers_dark(&settings));

        let provider_for_preference = provider.clone();
        settings.connect_gtk_application_prefer_dark_theme_notify(move |settings| {
            load_translation_empty_css(&provider_for_preference, settings_prefers_dark(settings));
        });

        let provider_for_theme = provider.clone();
        settings.connect_gtk_theme_name_notify(move |settings| {
            load_translation_empty_css(&provider_for_theme, settings_prefers_dark(settings));
        });
    } else {
        load_translation_empty_css(&provider, false);
    }

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn load_translation_empty_css(provider: &gtk4::CssProvider, is_dark: bool) {
    provider.load_from_data(translation_empty_css(is_dark));
}

fn translation_empty_css(is_dark: bool) -> &'static str {
    if is_dark {
        ".translation-empty text { background-color: #2d2d2d; }"
    } else {
        ".translation-empty text { background-color: #f8f8f8; }"
    }
}

fn settings_prefers_dark(settings: &gtk4::Settings) -> bool {
    settings.is_gtk_application_prefer_dark_theme()
        || settings
            .gtk_theme_name()
            .is_some_and(|theme| theme.to_ascii_lowercase().contains("dark"))
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
    language_entry: &ComboBoxText,
    preset_combo: &ComboBoxText,
    input_buffer: &TextBuffer,
    output_buffer: &TextBuffer,
    input_clear_button: &Button,
    output_clear_button: &Button,
    copy_button: &Button,
    settings_button: &Button,
    close_button: &Button,
    translate_button: &Button,
    status_label: &Label,
    _config: AppConfig,
    store: ConfigStore,
    secrets: impl SecretStore + Clone + Send + Sync + 'static,
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
    let store_for_settings = store.clone();
    let secrets_for_settings = secrets.clone();
    settings_button.connect_clicked(move |_| {
        let parent = parent_for_settings.clone();
        let config = runtime_for_settings.config();
        let store = store_for_settings.clone();
        let secrets = secrets_for_settings.clone();
        let runtime = runtime_for_settings.clone();
        glib::MainContext::default().spawn_local(async move {
            let dialog = SettingsDialog::build(&parent, store, secrets, config, runtime).await;
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

    let input_for_clear = input_buffer.clone();
    input_clear_button.connect_clicked(move |_| {
        input_for_clear.set_text("");
    });

    let output_for_clear = output_buffer.clone();
    let copy_button_for_clear = copy_button.clone();
    output_clear_button.connect_clicked(move |_| {
        output_for_clear.set_text("");
        copy_button_for_clear.set_sensitive(false);
    });

    let runtime_for_translate = runtime.clone();
    let input_for_translate = input_buffer.clone();
    let language_for_translate = language_entry.clone();
    let preset_for_translate = preset_combo.clone();
    let translate_button_for_translate = translate_button.clone();
    let copy_button_for_translate = copy_button.clone();
    let status_for_translate = status_label.clone();
    let output_for_translate = output_buffer.clone();
    let secrets_for_translate = secrets.clone();
    translate_button.connect_clicked(move |_| {
        let input = input_for_translate
            .text(
                &input_for_translate.start_iter(),
                &input_for_translate.end_iter(),
                false,
            )
            .to_string();
        if input.trim().is_empty() {
            status_for_translate.set_text("Enter text to translate.");
            return;
        }
        let language = language_for_translate
            .child()
            .and_downcast::<Entry>()
            .map(|e| e.text().to_string())
            .unwrap_or_default();
        if language.trim().is_empty() {
            status_for_translate.set_text("Enter target language.");
            return;
        }
        let Some(preset_id) = preset_for_translate.active_id().map(|id| id.to_string()) else {
            status_for_translate.set_text("Select a preset.");
            return;
        };

        translate_button_for_translate.set_sensitive(false);
        status_for_translate.set_text("Translating…");
        copy_button_for_translate.set_sensitive(false);
        output_for_translate.set_text("");

        let (sender, receiver) = mpsc::channel();
        let runtime = runtime_for_translate.clone();
        let secrets = secrets_for_translate.clone();
        tokio::spawn(async move {
            let outcome = runtime
                .translate_text(&secrets, &NotifySend, &input, &language, &preset_id)
                .await;
            let _ = sender.send(outcome);
        });

        let output_buffer = output_for_translate.clone();
        let copy_button = copy_button_for_translate.clone();
        let translate_button = translate_button_for_translate.clone();
        let status_label = status_for_translate.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || match receiver
            .try_recv()
        {
            Ok(Ok(outcome)) => {
                if let Some(translated_text) = outcome.translated_text {
                    output_buffer.set_text(&translated_text);
                    status_label.set_text("");
                    copy_button.set_sensitive(true);
                } else if let Some(message) = outcome.message {
                    status_label.set_text(&message);
                    copy_button.set_sensitive(false);
                }
                translate_button.set_sensitive(true);
                glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                status_label.set_text(&err.to_string());
                copy_button.set_sensitive(false);
                translate_button.set_sensitive(true);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                status_label.set_text("Translation task stopped.");
                copy_button.set_sensitive(false);
                translate_button.set_sensitive(true);
                glib::ControlFlow::Break
            }
        });
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

#[cfg(test)]
mod tests {
    use super::translation_empty_css;

    #[test]
    fn translation_empty_css_should_use_light_empty_background() {
        assert_eq!(
            translation_empty_css(false),
            ".translation-empty text { background-color: #f8f8f8; }"
        );
    }

    #[test]
    fn translation_empty_css_should_use_dark_empty_background() {
        assert_eq!(
            translation_empty_css(true),
            ".translation-empty text { background-color: #2d2d2d; }"
        );
    }
}
