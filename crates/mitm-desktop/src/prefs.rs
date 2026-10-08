use eframe::Storage;
use egui_dock::DockState;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use crate::dock::{self, Tab};
use crate::features::collections::CollectionsPrefs;
use crate::features::inspector::InspectorPrefs;
use crate::features::repeater::RepeaterPrefs;
use crate::features::settings::SettingsPrefs;
use crate::features::traffic::TrafficPrefs;
use crate::state::AppState;

// Separate keys, so a section that stops parsing resets alone instead of wiping the rest.
trait Section: Serialize + DeserializeOwned + Default {
    const KEY: &'static str;
}

#[derive(Serialize, Deserialize, Default)]
struct DockLayout(Option<DockState<Tab>>);

impl Section for DockLayout {
    const KEY: &'static str = "dock";
}

impl Section for InspectorPrefs {
    const KEY: &'static str = "inspector";
}

impl Section for RepeaterPrefs {
    const KEY: &'static str = "repeater";
}

impl Section for SettingsPrefs {
    const KEY: &'static str = "settings";
}

impl Section for TrafficPrefs {
    const KEY: &'static str = "traffic";
}

impl Section for CollectionsPrefs {
    const KEY: &'static str = "collections";
}

fn load<T: Section>(storage: Option<&dyn Storage>) -> T {
    storage.and_then(|s| eframe::get_value(s, T::KEY)).unwrap_or_default()
}

fn save<T: Section>(storage: &mut dyn Storage, value: &T) {
    eframe::set_value(storage, T::KEY, value);
}

pub fn restore(storage: Option<&dyn Storage>, state: &mut AppState) -> DockState<Tab> {
    state.inspector.prefs = load(storage);
    state.repeater.prefs = load(storage);
    state.collections.prefs = load(storage);
    state.settings = load(storage);
    state.traffic.prefs = load(storage);
    load::<DockLayout>(storage).0.unwrap_or_else(dock::initial_layout)
}

pub fn carry_over(from: &mut AppState, to: &mut AppState) {
    to.inspector.prefs = std::mem::take(&mut from.inspector.prefs);
    to.repeater.prefs = std::mem::take(&mut from.repeater.prefs);
    to.collections.prefs = std::mem::take(&mut from.collections.prefs);
    to.settings = std::mem::take(&mut from.settings);
    to.traffic.prefs = std::mem::take(&mut from.traffic.prefs);
}

pub fn persist(storage: &mut dyn Storage, dock: &DockState<Tab>, state: &AppState) {
    save(storage, &DockLayout(Some(dock.clone())));
    save(storage, &state.inspector.prefs);
    save(storage, &state.repeater.prefs);
    save(storage, &state.collections.prefs);
    save(storage, &state.settings);
    save(storage, &state.traffic.prefs);
}
