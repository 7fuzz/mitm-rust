use crate::{fuzzer, MitmCore};

impl MitmCore {
    pub fn clear_fuzz_runs(&self) -> Result<(), String> {
        fuzzer::clear_fuzz_runs(&self.db_path)
    }

    /// Starts an attack in the background. Results arrive as `CoreEvent::FuzzResult` once they're stored,
    /// then `CoreEvent::FuzzFinished`. The previous unsaved run is discarded.
    pub async fn start_fuzz(&self, config: fuzzer::FuzzConfig) -> Result<fuzzer::FuzzStart, String> {
        let (names, combos) = fuzzer::generate_combos(&config)?;
        let cancel = self.state.fuzz.begin()?;
        let run_id = self.state.next_fuzz_run_id();
        let begun = fuzzer::discard_unsaved_runs_db(&self.db_path)
            .and_then(|_| fuzzer::begin_run_db(&self.db_path, run_id, &config, combos.len()));
        if let Err(e) = begun {
            cancel.cancel();
            return Err(e);
        }
        let start = fuzzer::FuzzStart { run_id, names: names.clone(), total: combos.len() };
        let (bus, db_path) = (self.event_bus.clone(), self.db_path.clone());
        tokio::spawn(fuzzer::execute::run_attack(bus, db_path, run_id, config, names, combos, cancel));
        Ok(start)
    }

    pub fn stop_fuzz(&self) {
        self.state.fuzz.stop();
    }

    pub fn fuzz_result(&self, run_id: i64, idx: u32) -> Result<Option<fuzzer::FuzzResult>, String> {
        fuzzer::get_result_db(&self.db_path, run_id, idx)
    }

    pub fn fuzz_rows(&self, run_id: i64) -> Result<Vec<fuzzer::FuzzRow>, String> {
        fuzzer::get_run_rows_db(&self.db_path, run_id)
    }

    pub fn search_fuzz_responses(&self, run_id: i64, query: &str) -> Result<Vec<u32>, String> {
        fuzzer::search_responses_db(&self.db_path, run_id, query)
    }

    pub fn save_fuzz_run(&self, id: i64, name: &str) -> Result<(), String> {
        fuzzer::save_run_db(&self.db_path, id, name)
    }

    pub fn is_fuzz_run_saved(&self, id: i64) -> Result<bool, String> {
        fuzzer::is_run_saved_db(&self.db_path, id)
    }

    pub fn list_fuzz_runs(&self) -> Result<Vec<fuzzer::FuzzRunMeta>, String> {
        fuzzer::list_runs_db(&self.db_path)
    }

    pub fn load_fuzz_run(&self, id: i64) -> Result<fuzzer::LoadedFuzzRun, String> {
        fuzzer::load_run_db(&self.db_path, id)
    }

    pub fn delete_fuzz_run(&self, id: i64) -> Result<(), String> {
        fuzzer::delete_run_db(&self.db_path, id)
    }
}
