/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DemoApp {
    task_pool_size: usize,
    value: f32,
    #[serde(skip)]
    task: Option<task_simple::Task<crate::DoublingFunction>>,
    #[serde(skip)]
    task_pool: Option<task_simple::TaskPool<crate::FileSizeFunction>>,
    #[serde(skip)]
    ongoing: Vec<task_simple::Ticket>,
    #[serde(skip)]
    files: Vec<crate::FileSize>,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            task_pool_size: 3,
            value: 2.7,
            task: Default::default(),
            task_pool: Default::default(),
            ongoing: Default::default(),
            files: Default::default(),
        }
    }
}

impl DemoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let previous = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        Self {
            task: Some(task_simple::Task::new("doubling_worker")),
            task_pool: Some(task_simple::TaskPool::new(
                "file_worker",
                previous.task_pool_size,
            )),
            ..previous
        }
    }
}

impl eframe::App for DemoApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for file in ctx.input_mut(|x| std::mem::take(&mut x.raw.dropped_files)) {
            self.ongoing
                .push(self.task_pool.as_mut().unwrap().enqueue(file));
        }
        if let Some(update) = self.task.as_mut().unwrap().check() {
            log::debug!("Received update: {update:?}");
            self.value = update as _;
        }
        for ticket in std::mem::take(&mut self.ongoing) {
            match self.task_pool.as_mut().unwrap().check(ticket) {
                task_simple::JobState::Ongoing(ticket) => self.ongoing.push(ticket),
                task_simple::JobState::Done(file_size) => {
                    log::debug!("Received update: {file_size:?}");
                    self.files.push(file_size);
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Task pool size
            {
                ui.label("Task Pool size is set once at App start");
                ui.add(egui::Slider::new(&mut self.task_pool_size, 1..=10).text("Task Pool Size"));
            }
            ui.separator();
            // Value - single task
            {
                ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
                if ui.button("Increment single task").clicked() {
                    self.task.as_mut().unwrap().enqueue(self.value);
                    log::debug!("Task Message send {}", self.value);
                }
            }
            ui.separator();
            // Files - task pool
            {
                ui.label("You can drop files here");
                ui.label(&format!(
                    "Files being processed in background: {}",
                    self.ongoing.len()
                ));
                egui::Grid::new("files").num_columns(3).show(ui, |ui| {
                    {
                        ui.heading("File");
                        ui.heading("Bytes");
                        ui.heading("Computation");
                        ui.end_row();
                    }
                    for crate::FileSize {
                        name,
                        bytes,
                        computation_result,
                    } in &self.files
                    {
                        ui.label(name);
                        ui.label(&bytes.to_string());
                        ui.label(&computation_result.to_string());
                        ui.end_row();
                    }
                });
            }
        });
    }
}
