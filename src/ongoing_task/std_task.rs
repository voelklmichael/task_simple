use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use super::BackgroundFunction;

enum Input<Initial, Trigger> {
    Initial(Initial),
    Trigger(Trigger),
}

pub(super) struct BackgroundTaskStd<F: BackgroundFunction> {
    trigger: Sender<Input<F::InitialState, F::Trigger>>,
    event: Receiver<F::Event>,
    done_receiver: Receiver<()>,
    _thread: JoinHandle<()>,
}
impl<F: BackgroundFunction> BackgroundTaskStd<F> {
    pub(super) fn new(thread_name: &str, initial_state: F::InitialState) -> Self {
        let (input_sender, input_receiver) = channel();
        let (event_sender, event_receiver) = channel();
        let (done_sender, done_receiver) = channel();
        let thread = std::thread::Builder::new()
            .name(thread_name.into())
            .spawn(move || {
                let mut state = None;
                while let Ok(input) = input_receiver.recv() {
                    match input {
                        Input::Initial(initial) => {
                            state =
                                Some(F::initial_state(Default::default(), initial, |e| {
                                    let _=event_sender.send(e);
                                }))
                        }
                        Input::Trigger(trigger) => {
                            if let Some(initial_state) = &mut state {
                                F::trigger(initial_state,trigger, |e| {
                                    let _=event_sender.send(e);
                                })
                            } else {
                                unreachable!("Initial State not yet initialized - this is set already inside this function");
                            }
                        }
                    }
                    let r= done_sender.send(());
                    if r.is_err(){break;}
                }
            })
            .unwrap();
        let r = input_sender.send(Input::Initial(initial_state));
        assert!(r.is_ok());
        Self {
            trigger: input_sender,
            event: event_receiver,
            done_receiver,
            _thread: thread,
        }
    }
    pub(super) fn trigger(&self, trigger: F::Trigger) {
        let r = self.trigger.send(Input::Trigger(trigger));
        assert!(r.is_ok());
    }
    pub(super) fn event(&self) -> Option<F::Event> {
        self.event.try_recv().ok()
    }

    pub(crate) fn check_done(&self) -> bool {
        self.done_receiver.try_recv().is_ok()
    }
}
