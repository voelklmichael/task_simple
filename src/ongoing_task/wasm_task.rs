use std::collections::VecDeque;

use super::BackgroundFunction;
pub(super) struct BackgroundTaskWasm<F: BackgroundFunction> {
    event_update: std::rc::Rc<std::cell::Cell<VecDeque<F::Event>>>,
    done_update: std::rc::Rc<std::cell::Cell<VecDeque<super::Ongoing>>>,
    bridge: gloo_worker::WorkerBridge<WebWorkerBackground<F>>,
}
impl<F: BackgroundFunction> BackgroundTaskWasm<F> {
    pub(super) fn new(javascript_name: &str, initial_state: F::InitialState) -> Self {
        let event_update = std::rc::Rc::new(std::cell::Cell::new(VecDeque::default()));
        let done_update = std::rc::Rc::new(std::cell::Cell::new(VecDeque::default()));
        let event_sender = event_update.clone();
        let done_sender = done_update.clone();
        let bridge = <WebWorkerBackground<F> as gloo_worker::Spawnable>::spawner()
            .callback(move |response| {
                // TODO: this seems to be a data-race issue
                if let Some(event) = response {
                    let mut previous = done_sender.take();
                    previous.push_back(super::Ongoing::Ongoing);
                    done_sender.set(previous);
                    let mut previous = event_sender.take();
                    previous.push_back(event);
                    event_sender.set(previous);
                } else {
                    let mut previous = done_sender.take();
                    previous.push_back(super::Ongoing::NotOnging);
                    done_sender.set(previous);
                }
            })
            .spawn(&format!("./{javascript_name}.js"));
        bridge.send(Input::Initial(initial_state));
        Self {
            event_update,
            done_update,
            bridge,
        }
    }
    pub(super) fn trigger(&mut self, trigger: F::Trigger) {
        self.bridge.send(Input::Trigger(trigger));
    }
    pub(super) fn event(&self) -> Option<F::Event> {
        let d = self.event_update.as_ref();
        d.take().pop_front()
    }
    pub(super) fn check_done(&self) -> Option<super::Ongoing> {
        let d = self.done_update.as_ref();
        d.take().pop_front()
    }
}
/// This is a webworker running the Function F::call
#[derive(Debug)]
pub struct WebWorkerBackground<F: BackgroundFunction> {
    function: std::marker::PhantomData<F>,
    state: Option<F::State>,
}
impl<F: BackgroundFunction> Default for WebWorkerBackground<F> {
    fn default() -> Self {
        Self {
            function: Default::default(),
            state: None,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Input<Initial, Trigger> {
    Initial(Initial),
    Trigger(Trigger),
}

impl<F: BackgroundFunction> gloo_worker::Worker for WebWorkerBackground<F> {
    type Message = std::convert::Infallible;
    type Input = Input<F::InitialState, F::Trigger>;
    type Output = Option<F::Event>;

    fn create(_scope: &gloo_worker::WorkerScope<Self>) -> Self {
        Default::default()
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
        match msg {
            Input::Initial(initial_state) => {
                self.state = Some(F::initial_state(F::default(), initial_state, |e| {
                    scope.respond(id, Some(e))
                }));
                scope.respond(id, None);
            }
            Input::Trigger(trigger) => {
                if let Some(state) = &mut self.state {
                    F::trigger(state, trigger, |e| scope.respond(id, Some(e)))
                } else {
                    unreachable!("Initial State not yet initialized - this is set already inside this function");
                }
            }
        }
    }
}
