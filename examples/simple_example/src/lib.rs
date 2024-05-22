#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::DemoApp;

#[derive(Default)]
pub struct DoublingFunction {}
impl task_simple::Function for DoublingFunction {
    type Input = f32;
    type Output = f64;
    fn call(&mut self, input: Self::Input) -> Self::Output {
        doubling(input)
    }
}
pub fn doubling(x: f32) -> f64 {
    x as f64 * 0.92345
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct FileSize {
    name: String,
    bytes: usize,
    computation_result: u16,
}
impl FileSize {
    fn new(
        egui::DroppedFile {
            path,
            name,
            mime: _,
            last_modified: _,
            bytes,
        }: egui::DroppedFile,
    ) -> Self {
        let bytes = if let Some(path) = path {
            std::fs::read(path).unwrap_or_default()
        } else {
            bytes.unwrap().to_vec()
        };
        let computation_result = bytes
            .iter()
            .cloned()
            .map(|x| x as u16)
            .fold(0u16, |previous, byte| {
                previous.wrapping_mul(byte).wrapping_add(byte)
            });
        Self {
            name,
            bytes: bytes.len(),
            computation_result,
        }
    }
}
#[derive(Default)]
pub struct FileSizeFunction {}
impl task_simple::Function for FileSizeFunction {
    type Input = egui::DroppedFile;
    type Output = FileSize;
    fn call(&mut self, input: Self::Input) -> Self::Output {
        FileSize::new(input)
    }
}
