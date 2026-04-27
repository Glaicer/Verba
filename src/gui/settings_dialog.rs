use std::{cell::RefCell, rc::Rc};

use gtk4::{
    prelude::*, ApplicationWindow, Box as GtkBox, Button, Dialog, Entry, Label, Orientation,
    PasswordEntry, ResponseType,
};

use crate::{
    app_runtime::AppRuntime,
    config::{AppConfig, ConfigStore, Preset},
    error::{Result, VerbaError},
    gui::preset_editor::PresetEditor,
    secrets::SecretStore,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiKeyEdit {
    Unchanged,
    Replace(String),
    Clear,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsDraft {
    pub base_url: String,
    pub model_name: String,
    pub api_key: ApiKeyEdit,
    pub presets: Vec<Preset>,
}

impl SettingsDraft {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            base_url: config.provider.base_url.clone(),
            model_name: config.provider.model_name.clone(),
            api_key: ApiKeyEdit::Unchanged,
            presets: config.presets.clone(),
        }
    }

    pub fn with_api_key(mut self, api_key: ApiKeyEdit) -> Self {
        self.api_key = api_key;
        self
    }

    pub fn validated_config(&self, mut config: AppConfig) -> Result<AppConfig> {
        if self.base_url.trim().is_empty() {
            return Err(VerbaError::Config(
                "LLM Provider base url is required".to_string(),
            ));
        }

        if self.model_name.trim().is_empty() {
            return Err(VerbaError::Config("model name is required".to_string()));
        }

        config.provider.base_url = self.base_url.trim().to_string();
        config.provider.model_name = self.model_name.trim().to_string();
        config.presets = self.presets.clone();
        config.validate()?;
        Ok(config)
    }
}

#[derive(Clone, Debug)]
pub struct SettingsDialog {
    dialog: Dialog,
}

impl SettingsDialog {
    pub async fn build<S>(
        parent: &ApplicationWindow,
        store: ConfigStore,
        secrets: S,
        config: AppConfig,
        runtime: AppRuntime,
    ) -> Self
    where
        S: SecretStore + Clone + 'static,
    {
        let key_is_configured = secrets.get_api_key().await.ok().flatten().is_some();
        Self::build_with_key_state(parent, store, secrets, config, runtime, key_is_configured)
    }

    fn build_with_key_state<S>(
        parent: &ApplicationWindow,
        store: ConfigStore,
        secrets: S,
        config: AppConfig,
        runtime: AppRuntime,
        key_is_configured: bool,
    ) -> Self
    where
        S: SecretStore + Clone + 'static,
    {
        let dialog = Dialog::builder()
            .title("Settings")
            .modal(true)
            .transient_for(parent)
            .default_width(560)
            .default_height(320)
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Save", ResponseType::Accept);
        pad_dialog_buttons(&dialog);

        let base_url_entry = Entry::builder()
            .hexpand(true)
            .text(&config.provider.base_url)
            .build();
        let model_entry = Entry::builder()
            .hexpand(true)
            .text(&config.provider.model_name)
            .build();
        let api_key_entry = PasswordEntry::builder().hexpand(true).build();
        api_key_entry.set_placeholder_text(Some(if key_is_configured { "Configured" } else { "" }));

        let error_label = Label::new(None);
        error_label.add_css_class("error");
        error_label.set_xalign(0.0);

        let content = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        content.append(&field_row("LLM Provider base url", &base_url_entry));
        content.append(&field_row("Model Name", &model_entry));
        content.append(&field_row("API Key", &api_key_entry));
        let configure_presets_button = Button::with_label("Configure Presets");
        content.append(&configure_presets_button);
        content.append(&error_label);
        dialog.content_area().append(&content);

        let staged_presets = Rc::new(RefCell::new(config.presets.clone()));
        let staged_for_editor = staged_presets.clone();
        let parent_for_editor = parent.clone();
        configure_presets_button.connect_clicked(move |_| {
            let editor = PresetEditor::build(&parent_for_editor, staged_for_editor.clone());
            editor.present();
        });

        let config_for_response = config.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                let draft = SettingsDraft {
                    base_url: base_url_entry.text().to_string(),
                    model_name: model_entry.text().to_string(),
                    api_key: api_key_edit_from_entry(&api_key_entry),
                    presets: staged_presets.borrow().clone(),
                };

                let store = store.clone();
                let secrets = secrets.clone();
                let config = config_for_response.clone();
                let runtime = runtime.clone();
                let error_label = error_label.clone();
                let dialog = dialog.clone();
                glib::MainContext::default().spawn_local(async move {
                    match apply_settings(&store, &secrets, config, draft).await {
                        Ok(config) => {
                            runtime.update_config(config);
                            dialog.close();
                        }
                        Err(err) => error_label.set_text(&err.to_string()),
                    }
                });
            } else {
                dialog.close();
            }
        });

        Self { dialog }
    }

    pub fn present(&self) {
        self.dialog.present();
    }
}

pub async fn apply_settings<S>(
    store: &ConfigStore,
    secrets: &S,
    config: AppConfig,
    draft: SettingsDraft,
) -> Result<AppConfig>
where
    S: SecretStore,
{
    let config = draft.validated_config(config)?;

    match &draft.api_key {
        ApiKeyEdit::Unchanged => {}
        ApiKeyEdit::Replace(value) => secrets.set_api_key(value).await?,
        ApiKeyEdit::Clear => secrets.clear_api_key().await?,
    }

    store.save(&config)?;
    Ok(config)
}

fn api_key_edit_from_entry(entry: &PasswordEntry) -> ApiKeyEdit {
    let value = entry.text().trim().to_string();
    if value.is_empty() {
        ApiKeyEdit::Unchanged
    } else {
        ApiKeyEdit::Replace(value)
    }
}

fn field_row(label: &str, input: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    let label = Label::new(Some(label));
    label.set_width_chars(22);
    label.set_xalign(0.0);
    row.append(&label);
    row.append(input);
    row
}

fn pad_dialog_buttons(dialog: &Dialog) {
    for response in [ResponseType::Cancel, ResponseType::Accept] {
        if let Some(button) = dialog.widget_for_response(response) {
            button.set_margin_top(8);
            button.set_margin_bottom(12);
        }
    }

    if let Some(cancel) = dialog.widget_for_response(ResponseType::Cancel) {
        cancel.set_margin_end(4);
    }
    if let Some(save) = dialog.widget_for_response(ResponseType::Accept) {
        save.set_margin_start(4);
        save.set_margin_end(12);
    }
}
