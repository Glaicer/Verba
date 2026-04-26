use verba::{
    config::AppConfig,
    gui::{
        actions::{accelerators_for_action, GuiAction},
        application_args, application_id,
        main_window::{default_languages, selected_preset_index},
        present_window_on_startup,
    },
};

#[test]
fn gui_should_offer_common_default_languages() {
    assert_eq!(
        default_languages(),
        [
            "English",
            "Russian",
            "German",
            "French",
            "Spanish",
            "Chinese",
            "Japanese",
            "Korean",
            "Italian",
            "Portuguese"
        ]
    );
}

#[test]
fn gui_actions_should_expose_plan_shortcuts() {
    assert_eq!(
        accelerators_for_action(GuiAction::Translate),
        ["<Control>Return"]
    );
    assert_eq!(accelerators_for_action(GuiAction::Close), ["Escape"]);
    assert_eq!(
        accelerators_for_action(GuiAction::FocusLanguage),
        ["<Control>L"]
    );
    assert_eq!(
        accelerators_for_action(GuiAction::FocusPreset),
        ["<Control>P"]
    );
    assert_eq!(
        accelerators_for_action(GuiAction::CopyResult),
        ["<Control><Shift>C"]
    );
}

#[test]
fn preset_dropdown_should_select_last_configured_preset() {
    let config = AppConfig::default();

    assert_eq!(selected_preset_index(&config), Some(0));
}

#[test]
fn preset_dropdown_should_fall_back_to_first_preset_when_saved_id_is_missing() {
    let mut config = AppConfig::default();
    config.ui.last_preset_id = "missing".to_string();

    assert_eq!(selected_preset_index(&config), Some(0));
}

#[test]
fn gtk_application_args_should_not_forward_cli_subcommands_as_files() {
    assert_eq!(application_args(), ["verba"]);
}

#[test]
fn gtk_application_should_use_distinct_id_from_daemon_service() {
    assert_eq!(application_id(), "dev.aronov.Verba.Gui");
}

#[test]
fn gtk_application_should_start_minimized_to_tray() {
    assert!(!present_window_on_startup());
}
