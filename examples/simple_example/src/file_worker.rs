fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use task_simple::gloo_worker::Registrable;
        console_error_panic_hook::set_once();
        task_simple::WebWorker::<simple_example::FileSizeFunction>::registrar().register();
    }
}
