use std::{path::PathBuf, thread, time::Duration};

use ksni::{menu::StandardItem, Icon, MenuItem, Tray, TrayService};

use crate::app_runtime::{AppRuntime, AppState};

pub struct TrayIndicator {
    handle: Option<ksni::Handle<VerbaTray>>,
    sync_thread: Option<thread::JoinHandle<()>>,
}

impl TrayIndicator {
    pub fn start(runtime: AppRuntime) -> Self {
        let service = TrayService::new(VerbaTray::new(runtime));
        let handle = service.handle();
        service.spawn();
        let sync_thread = Some(spawn_state_sync(handle.clone()));
        Self {
            handle: Some(handle),
            sync_thread,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            handle: None,
            sync_thread: None,
        }
    }

    pub fn shutdown(&self) {
        if let Some(handle) = &self.handle {
            handle.shutdown();
        }
    }

    pub fn state_sync_for_tests(runtime: AppRuntime) -> TrayStateSync {
        TrayStateSync::new(runtime)
    }
}

impl Drop for TrayIndicator {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(sync_thread) = self.sync_thread.take() {
            let _ = sync_thread.join();
        }
    }
}

#[derive(Clone, Debug)]
pub struct VerbaTray {
    runtime: AppRuntime,
}

impl VerbaTray {
    pub fn new(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn icon_name_static() -> &'static str {
        "verba"
    }

    pub fn icon_theme_path_static() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packaging/icons")
    }

    pub fn icon_pixmaps_for_tests(&self) -> Vec<Icon> {
        verba_icon_pixmap()
    }

    pub fn menu_labels(&self) -> [&'static str; 3] {
        [self.open_or_minimize_label(), "Settings", "Exit"]
    }

    pub fn open_or_minimize_label(&self) -> &'static str {
        if self.runtime.main_window_visible() {
            "Minimize"
        } else {
            "Open"
        }
    }

    pub fn open_or_minimize(&mut self) {
        if self.runtime.main_window_visible() {
            self.runtime.hide_main_window();
        } else {
            self.runtime.show_main_window();
        }
    }

    pub fn left_click(&mut self) {
        self.runtime.toggle_main_window();
    }

    pub fn open_settings(&mut self) {
        self.runtime.open_settings();
    }

    pub fn exit(&mut self) {
        self.runtime.quit();
    }
}

impl Tray for VerbaTray {
    fn id(&self) -> String {
        "verba".to_string()
    }

    fn title(&self) -> String {
        "Verba".to_string()
    }

    fn icon_name(&self) -> String {
        Self::icon_name_static().to_string()
    }

    fn icon_theme_path(&self) -> String {
        let system_icon_dir = PathBuf::from("/usr/share/icons/hicolor/scalable/apps");
        if system_icon_dir.join("verba.svg").exists() {
            return system_icon_dir.to_string_lossy().into_owned();
        }
        Self::icon_theme_path_static()
            .to_string_lossy()
            .into_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        verba_icon_pixmap()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.left_click();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: self.open_or_minimize_label().to_string(),
                icon_name: "window".to_string(),
                activate: Box::new(|tray: &mut Self| tray.open_or_minimize()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Settings".to_string(),
                icon_name: "preferences-system".to_string(),
                activate: Box::new(|tray: &mut Self| tray.open_settings()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Exit".to_string(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(|tray: &mut Self| tray.exit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct TrayStateSync {
    runtime: AppRuntime,
    last_state: AppState,
}

impl TrayStateSync {
    fn new(runtime: AppRuntime) -> Self {
        let last_state = runtime.state();
        Self {
            runtime,
            last_state,
        }
    }

    pub fn poll_state_changed(&mut self) -> bool {
        let state = self.runtime.state();
        if state == self.last_state {
            return false;
        }

        self.last_state = state;
        true
    }
}

fn spawn_state_sync(handle: ksni::Handle<VerbaTray>) -> thread::JoinHandle<()> {
    let runtime = handle.update(|tray| tray.runtime.clone());
    thread::spawn(move || {
        let mut sync = TrayStateSync::new(runtime.clone());
        while !runtime.is_exiting() {
            if sync.poll_state_changed() {
                handle.update(|_| {});
            }
            thread::sleep(Duration::from_millis(100));
        }
        handle.update(|_| {});
    })
}

fn verba_icon_pixmap() -> Vec<Icon> {
    const SIZE: usize = 32;
    let mut data = vec![0_u8; SIZE * SIZE * 4];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let (a, r, g, b) = icon_pixel(x, y);
            let offset = (y * SIZE + x) * 4;
            data[offset] = a;
            data[offset + 1] = r;
            data[offset + 2] = g;
            data[offset + 3] = b;
        }
    }

    vec![Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data,
    }]
}

fn icon_pixel(x: usize, y: usize) -> (u8, u8, u8, u8) {
    let in_window = (3..=28).contains(&x) && (5..=26).contains(&y);
    if !in_window {
        return (0, 0, 0, 0);
    }

    if x == 3 || x == 28 || y == 5 || y == 26 {
        return (255, 31, 41, 55);
    }

    if (6..=9).contains(&y) {
        if y == 7 && matches!(x, 7 | 10 | 13) {
            return (255, 191, 219, 254);
        }
        return (255, 37, 99, 235);
    }

    if ((7..=14).contains(&x) && matches!(y, 14 | 18 | 22)) || ((15..=17).contains(&x) && y == 18) {
        return (255, 148, 163, 184);
    }

    if (18..=25).contains(&x) && (13..=23).contains(&y) {
        if (x == 21 && (13..=23).contains(&y))
            || (y == 16 && (18..=25).contains(&x))
            || (x + y == 41 && (20..=25).contains(&x))
            || (x + 3 == y + 18 && (18..=22).contains(&x) && (18..=22).contains(&y))
        {
            return (255, 37, 99, 235);
        }
        if (y == 20 && (18..=25).contains(&x)) || (x == 23 && (17..=23).contains(&y)) {
            return (255, 15, 23, 42);
        }
    }

    (255, 248, 250, 252)
}
