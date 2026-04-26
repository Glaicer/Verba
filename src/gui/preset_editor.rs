use std::{cell::RefCell, rc::Rc};

use gtk4::{
    prelude::*, ApplicationWindow, Box as GtkBox, Button, Dialog, Entry, Label, Orientation,
    ResponseType, ScrolledWindow, TextBuffer, TextView, WrapMode,
};
use uuid::Uuid;

use crate::{
    config::{AppConfig, Preset},
    error::{Result, VerbaError},
};

#[derive(Clone, Debug)]
pub struct PresetEditorModel {
    presets: Vec<Preset>,
}

impl PresetEditorModel {
    pub fn new(presets: Vec<Preset>) -> Self {
        Self { presets }
    }

    pub fn presets(&self) -> &[Preset] {
        &self.presets
    }

    pub fn into_presets(self) -> Vec<Preset> {
        self.presets
    }

    pub fn with_added_preset(mut self, name: &str, instruction: &str) -> Result<Self> {
        self.add_preset(name, instruction)?;
        Ok(self)
    }

    pub fn add_preset(&mut self, name: &str, instruction: &str) -> Result<()> {
        let name = name.trim();
        let instruction = instruction.trim();
        if name.is_empty() {
            return Err(VerbaError::Config("preset name is required".to_string()));
        }
        if instruction.is_empty() {
            return Err(VerbaError::Config(
                "preset instruction is required".to_string(),
            ));
        }

        self.presets.push(Preset {
            id: self.unique_id_for_name(name),
            name: name.to_string(),
            instruction: instruction.to_string(),
        });
        Ok(())
    }

    pub fn delete_preset(&mut self, id: &str) -> Result<bool> {
        if self.presets.len() == 1 && self.presets[0].id == id {
            return Err(VerbaError::Config(
                "last preset cannot be deleted".to_string(),
            ));
        }

        let before = self.presets.len();
        self.presets.retain(|preset| preset.id != id);
        Ok(self.presets.len() != before)
    }

    pub fn validate(&self) -> Result<()> {
        let mut config = AppConfig::default();
        config.presets = self.presets.clone();
        config.validate()
    }

    fn unique_id_for_name(&self, name: &str) -> String {
        let base = slugify(name).unwrap_or_else(|| Uuid::new_v4().to_string());
        if !self.presets.iter().any(|preset| preset.id == base) {
            return base;
        }

        for suffix in 2.. {
            let candidate = format!("{base}-{suffix}");
            if !self.presets.iter().any(|preset| preset.id == candidate) {
                return candidate;
            }
        }

        unreachable!("unbounded suffix loop should always return")
    }
}

#[derive(Clone, Debug)]
pub struct PresetEditor {
    dialog: Dialog,
}

impl PresetEditor {
    pub fn build(parent: &ApplicationWindow, staged_presets: Rc<RefCell<Vec<Preset>>>) -> Self {
        let dialog = Dialog::builder()
            .title("Configure Presets")
            .modal(true)
            .transient_for(parent)
            .default_width(720)
            .default_height(520)
            .build();
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Save", ResponseType::Accept);

        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let rows = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .build();
        let row_widgets: Rc<RefCell<Vec<PresetRow>>> = Rc::default();
        populate_rows(&rows, &row_widgets, &staged_presets.borrow());

        let scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .min_content_height(360)
            .build();
        scroll.set_child(Some(&rows));

        let add_button = Button::with_label("Add");
        let error_label = Label::new(None);
        error_label.add_css_class("error");
        error_label.set_xalign(0.0);

        root.append(&scroll);
        root.append(&add_button);
        root.append(&error_label);
        dialog.content_area().append(&root);

        let rows_for_add = rows.clone();
        let row_widgets_for_add = row_widgets.clone();
        add_button.connect_clicked(move |_| {
            append_row(
                &rows_for_add,
                &row_widgets_for_add,
                Preset {
                    id: Uuid::new_v4().to_string(),
                    name: "New Preset".to_string(),
                    instruction: "Describe the translation style.".to_string(),
                },
            );
        });

        let staged_for_response = staged_presets.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                match collect_rows(&row_widgets.borrow()) {
                    Ok(presets) => {
                        *staged_for_response.borrow_mut() = presets;
                        dialog.close();
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
struct PresetRow {
    id: String,
    container: GtkBox,
    name_entry: Entry,
    instruction_buffer: TextBuffer,
}

fn populate_rows(rows: &GtkBox, row_widgets: &Rc<RefCell<Vec<PresetRow>>>, presets: &[Preset]) {
    for preset in presets {
        append_row(rows, row_widgets, preset.clone());
    }
}

fn append_row(rows: &GtkBox, row_widgets: &Rc<RefCell<Vec<PresetRow>>>, preset: Preset) {
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    let name_entry = Entry::builder().hexpand(true).text(&preset.name).build();
    let delete_button = Button::with_label("Delete");
    header.append(&Label::new(Some("Name")));
    header.append(&name_entry);
    header.append(&delete_button);

    let instruction_buffer = TextBuffer::new(None);
    instruction_buffer.set_text(&preset.instruction);
    let instruction_view = TextView::builder()
        .buffer(&instruction_buffer)
        .wrap_mode(WrapMode::WordChar)
        .vexpand(false)
        .height_request(96)
        .build();
    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .min_content_height(96)
        .build();
    scroll.set_child(Some(&instruction_view));

    container.append(&header);
    container.append(&Label::new(Some("Instruction")));
    container.append(&scroll);
    rows.append(&container);

    let row = PresetRow {
        id: preset.id,
        container: container.clone(),
        name_entry,
        instruction_buffer,
    };
    row_widgets.borrow_mut().push(row);

    let row_widgets_for_delete = row_widgets.clone();
    delete_button.connect_clicked(move |_| {
        let rows = row_widgets_for_delete.borrow();
        if rows.len() <= 1 {
            return;
        }
        drop(rows);

        let mut rows = row_widgets_for_delete.borrow_mut();
        if let Some(index) = rows.iter().position(|row| row.container == container) {
            let row = rows.remove(index);
            row.container.unparent();
        }
    });
}

fn collect_rows(rows: &[PresetRow]) -> Result<Vec<Preset>> {
    let mut presets = Vec::with_capacity(rows.len());
    for row in rows {
        let name = row.name_entry.text().trim().to_string();
        let instruction = row
            .instruction_buffer
            .text(
                &row.instruction_buffer.start_iter(),
                &row.instruction_buffer.end_iter(),
                false,
            )
            .trim()
            .to_string();

        presets.push(Preset {
            id: row.id.clone(),
            name,
            instruction,
        });
    }

    let model = PresetEditorModel::new(presets);
    model.validate()?;
    Ok(model.into_presets())
}

fn slugify(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}
