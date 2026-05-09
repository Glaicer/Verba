use std::{cell::RefCell, rc::Rc};

use gtk4::{
    prelude::*, ApplicationWindow, Box as GtkBox, Button, Dialog, Entry, Label, Orientation,
    ResponseType, ScrolledWindow,
};

use crate::{
    app_runtime::AppRuntime,
    config::{AppConfig, ConfigStore},
    error::Result,
};

#[derive(Clone, Debug)]
pub struct LanguageEditorModel {
    languages: Vec<String>,
}

impl LanguageEditorModel {
    pub fn new(languages: Vec<String>) -> Self {
        Self { languages }
    }

    pub fn validate(&self) -> Result<()> {
        let mut config = AppConfig::default();
        config.languages = self.languages.clone();
        config.validate()
    }
}

#[derive(Clone, Debug)]
pub struct LanguageEditor {
    dialog: Dialog,
}

impl LanguageEditor {
    pub fn build(
        parent: &ApplicationWindow,
        store: ConfigStore,
        runtime: AppRuntime,
        config: AppConfig,
    ) -> Self {
        let dialog = Dialog::builder()
            .title("Configure Languages")
            .modal(true)
            .transient_for(parent)
            .default_width(480)
            .default_height(420)
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Save", ResponseType::Accept);
        pad_dialog_buttons(&dialog);

        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(16)
            .margin_start(12)
            .margin_end(16)
            .build();

        let rows = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .build();
        let row_widgets: Rc<RefCell<Vec<LanguageRow>>> = Rc::default();
        populate_rows(&rows, &row_widgets, &config.languages);

        let scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .min_content_height(300)
            .build();
        scroll.set_child(Some(&rows));

        let add_button = Button::with_label("Add");
        add_button.set_halign(gtk4::Align::Center);
        add_button.set_width_request(96);
        let error_label = Label::new(None);
        error_label.add_css_class("error");
        error_label.set_xalign(0.0);

        root.append(&scroll);
        root.append(&add_button);
        root.append(&error_label);
        dialog.content_area().append(&root);

        let rows_for_add = rows.clone();
        let row_widgets_for_add = row_widgets.clone();
        let scroll_for_add = scroll.clone();
        add_button.connect_clicked(move |_| {
            append_row(&rows_for_add, &row_widgets_for_add, String::new());
            let adjustment = scroll_for_add.vadjustment();
            glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                adjustment.set_value(adjustment.upper() - adjustment.page_size());
            });
        });

        let store_for_response = store;
        let runtime_for_response = runtime;
        let config_for_response = config;
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                let languages = collect_languages(&row_widgets.borrow());
                let model = LanguageEditorModel::new(languages);
                match model.validate() {
                    Ok(()) => {
                        let mut config = config_for_response.clone();
                        config.languages = model.languages.clone();
                        match store_for_response.save(&config) {
                            Ok(()) => {
                                runtime_for_response.update_config(config);
                                dialog.close();
                            }
                            Err(err) => error_label.set_text(&err.to_string()),
                        }
                    }
                    Err(err) => error_label.set_text(&err.to_string()),
                }
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

#[derive(Clone)]
struct LanguageRow {
    container: GtkBox,
    entry: Entry,
}

fn populate_rows(
    rows: &GtkBox,
    row_widgets: &Rc<RefCell<Vec<LanguageRow>>>,
    languages: &[String],
) {
    for lang in languages {
        append_row(rows, row_widgets, lang.clone());
    }
}

fn append_row(rows: &GtkBox, row_widgets: &Rc<RefCell<Vec<LanguageRow>>>, language: String) {
    let container = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_end(12)
        .build();

    let entry = Entry::builder().hexpand(true).text(&language).build();
    let remove_button = Button::with_label("Remove");
    container.append(&entry);
    container.append(&remove_button);
    rows.append(&container);

    let row = LanguageRow {
        container: container.clone(),
        entry,
    };
    row_widgets.borrow_mut().push(row);

    let row_widgets_for_remove = row_widgets.clone();
    remove_button.connect_clicked(move |_| {
        let rows = row_widgets_for_remove.borrow();
        if rows.len() <= 1 {
            return;
        }
        drop(rows);

        let mut rows = row_widgets_for_remove.borrow_mut();
        if let Some(index) = rows.iter().position(|r| r.container == container) {
            let row = rows.remove(index);
            row.container.unparent();
        }
    });
}

fn collect_languages(rows: &[LanguageRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.entry.text().to_string())
        .collect()
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
