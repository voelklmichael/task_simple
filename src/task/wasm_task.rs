use std::collections::VecDeque;

use super::Function;
pub(super) struct TaskWasm<F: Function> {
    data_update: std::rc::Rc<std::cell::Cell<VecDeque<F::Output>>>,
    bridge: gloo_worker::WorkerBridge<WebWorker<F>>,
}
impl<F: Function> TaskWasm<F> {
    pub(super) fn new(javascript_name: &str) -> Self {
        let data_update = std::rc::Rc::new(std::cell::Cell::new(VecDeque::default()));
        let sender = data_update.clone();
        let bridge = <WebWorker<F> as gloo_worker::Spawnable>::spawner()
            .callback(move |response| {
                // TODO: this seems to be a data-race issue
                let mut previous = sender.take();
                previous.push_back(response);
                sender.set(previous);
            })
            .spawn(&format!("./{javascript_name}.js"));
        Self {
            data_update,
            bridge,
        }
    }
    pub(super) fn enqueue(&mut self, msg: F::Input) {
        self.bridge.send(msg);
    }
    pub(super) fn check(&self) -> Option<F::Output> {
        let d = self.data_update.as_ref();
        d.take().pop_front()
    }
}
/// This is a webworker running the Function F::call
#[derive(Debug)]
pub struct WebWorker<F>(F);

impl<F: Function> gloo_worker::Worker for WebWorker<F> {
    type Message = std::convert::Infallible;
    type Input = F::Input;
    type Output = F::Output;

    fn create(_scope: &gloo_worker::WorkerScope<Self>) -> Self {
        Self(Default::default())
    }

    fn update(&mut self, _scope: &gloo_worker::WorkerScope<Self>, msg: Self::Message) {
        match msg {}
    }

    fn received(
        &mut self,
        scope: &gloo_worker::WorkerScope<Self>,
        msg: Self::Input,
        id: gloo_worker::HandlerId,
    ) {
        let output = self.0.call(msg);
        scope.respond(id, output);
    }
}
