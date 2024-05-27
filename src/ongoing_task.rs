#[cfg(not(target_arch = "wasm32"))]
mod std_task;
#[cfg(target_arch = "wasm32")]
mod wasm_task;

#[cfg(target_arch = "wasm32")]
pub use wasm_task::WebWorkerBackground;

/// This trait abstracts a function, which can be run independently
pub trait BackgroundFunction: 'static + Default + Sized {
    /// Initial State of background thread
    /// Note: This is runtime-dependent, because it typically is deserialized
    type InitialState: serde::Serialize + serde::de::DeserializeOwned + Send;
    /// Current State of function
    type State: StateTrait<Event = Self::Event>;
    /// An outer event which is forwarded to the background task
    type Trigger: serde::Serialize + serde::de::DeserializeOwned + Send;
    /// An event produced by the background task
    type Event: serde::Serialize + serde::de::DeserializeOwned + Send;
    /// Function to initialize state
    /// Event sender sends 'None' once this is finished
    fn initial_state<EventSender: Fn(Self::Event)>(
        self,
        initial_state: Self::InitialState,
        event_sender: EventSender,
    ) -> Self::State;
    /// Function to trigger state
    fn trigger<EventSender: Fn(Self::Event)>(
        state: &mut Self::State,
        trigger: Self::Trigger,
        event_sender: EventSender,
    );
    /// Merge two events
    fn event_merge(event: &mut Self::Event, other: Self::Event);
}

#[derive(Debug)]
/// The internal state of the background task
pub enum StateProgress<T> {
    /// The background task is waiting for external input
    NothingOngoing,
    /// The backgronud task is computing something
    Ongoing,
    /// The background task has a message for the external world
    Event(T),
}

/// This trait has to be implemented by the background task's state
pub trait StateTrait {
    /// Event type for the external world
    type Event;
    /// Check the progress of the background task
    fn progress(&mut self) -> StateProgress<Self::Event>;
}

/// This is a long running background task
pub struct BackgroundTask<F: BackgroundFunction> {
    task_ongoing: Ongoing,
    #[cfg(not(target_arch = "wasm32"))]
    background_task: std_task::BackgroundTaskStd<F>,
    #[cfg(target_arch = "wasm32")]
    background_task: wasm_task::BackgroundTaskWasm<F>,
}
impl<F: BackgroundFunction> std::fmt::Debug for BackgroundTask<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundTask").finish()
    }
}
#[derive(PartialEq, Debug)]
enum Ongoing {
    NotOnging,
    Ongoing,
}
impl<F: BackgroundFunction> BackgroundTask<F> {
    /// Start a new long running backround task in the background.
    #[must_use]
    pub fn new(task_name: &str, initial_state: F::InitialState) -> Self {
        Self {
            task_ongoing: Ongoing::Ongoing,
            background_task: {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    std_task::BackgroundTaskStd::new(task_name, initial_state)
                }
                #[cfg(target_arch = "wasm32")]
                {
                    wasm_task::BackgroundTaskWasm::new(task_name, initial_state)
                }
            },
        }
    }
    /// Trigger a new action.
    pub fn trigger(&mut self, trigger: F::Trigger) {
        self.task_ongoing = Ongoing::Ongoing;
        self.background_task.trigger(trigger);
    }

    /// Check if some action is ongoing
    #[must_use]
    pub fn is_ongoing(&mut self) -> bool {
        while let Some(ongoing) = self.background_task.check_done() {
            self.task_ongoing = ongoing;
        }
        self.task_ongoing == Ongoing::Ongoing
    }

    /// Fetch collected events
    #[must_use]
    pub fn event(&mut self) -> Option<F::Event> {
        match self.background_task.event() {
            Some(mut event) => {
                while let Some(other) = self.background_task.event() {
                    F::event_merge(&mut event, other);
                }
                Some(event)
            }
            None => None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_background_task_std_simple() {
    struct State(f32);
    impl StateTrait for State {
        type Event = f64;
        fn progress(&mut self) -> StateProgress<Self::Event> {
            StateProgress::NothingOngoing
        }
    }
    #[derive(Default)]
    struct DummyFunction;
    impl BackgroundFunction for DummyFunction {
        type InitialState = ();
        type State = State;
        type Trigger = f32;
        type Event = f64;

        fn initial_state<EventSender: Fn(Self::Event)>(
            self,
            (): Self::InitialState,
            event_sender: EventSender,
        ) -> Self::State {
            event_sender(1.);
            std::thread::sleep(std::time::Duration::from_secs(1));
            State(42.)
        }

        fn trigger<EventSender: Fn(Self::Event)>(
            state: &mut Self::State,
            trigger: Self::Trigger,
            event_sender: EventSender,
        ) {
            (0..trigger.abs().ceil() as usize).for_each(|x| event_sender(x as _));
            std::thread::sleep(std::time::Duration::from_secs(1));
            state.0 += trigger;
        }

        fn event_merge(event: &mut Self::Event, other: Self::Event) {
            *event = event.max(other)
        }
    }

    let mut task = BackgroundTask::<DummyFunction>::new("dummy_thread", ());
    while task.is_ongoing() {}
    let event = dbg!(task.event());
    assert_eq!(event, Some(1.));
    task.trigger(2.4);
    while task.is_ongoing() {}
    let event: Option<f64> = dbg!(task.event());
    assert_eq!(event, Some(2.));
    task.trigger(3.4);
    task.trigger(2.4);
    while task.is_ongoing() {}
    let event: Option<f64> = dbg!(task.event());
    assert_eq!(event, Some(3.));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_background_task_std_complex() {
    use std::thread::JoinHandle;
    struct State(Vec<JoinHandle<f32>>);
    impl StateTrait for State {
        type Event = f64;
        fn progress(&mut self) -> StateProgress<Self::Event> {
            for (i, handle) in self.0.iter_mut().enumerate() {
                if handle.is_finished() {
                    let handle = self.0.remove(i);
                    let output = handle.join().unwrap();
                    return StateProgress::Event(output as f64);
                }
            }
            if self.0.is_empty() {
                StateProgress::NothingOngoing
            } else {
                StateProgress::Ongoing
            }
        }
    }
    #[derive(Default)]
    struct DummyFunction;
    impl BackgroundFunction for DummyFunction {
        type InitialState = ();
        type State = State;
        type Trigger = f32;
        type Event = f64;

        fn initial_state<EventSender: Fn(Self::Event)>(
            self,
            (): Self::InitialState,
            event_sender: EventSender,
        ) -> Self::State {
            event_sender(1.);
            let thread = std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(2));
                42.
            });

            std::thread::sleep(std::time::Duration::from_secs(1));
            State(vec![thread])
        }

        fn trigger<EventSender: Fn(Self::Event)>(
            state: &mut Self::State,
            trigger: Self::Trigger,
            event_sender: EventSender,
        ) {
            (0..trigger.abs().ceil() as usize).for_each(|x| event_sender(x as _));
            let n = state.0.len();
            let thread = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                trigger + n as f32
            });
            state.0.push(thread);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        fn event_merge(event: &mut Self::Event, other: Self::Event) {
            *event = event.max(other)
        }
    }

    let mut task = BackgroundTask::<DummyFunction>::new("dummy_thread", ());
    while task.is_ongoing() {}
    let event = dbg!(task.event());
    assert_eq!(event, Some(42.));
    task.trigger(2.);
    while task.is_ongoing() {}
    let event: Option<f64> = dbg!(task.event());
    assert_eq!(event, Some(2.));
    task.trigger(3.);
    task.trigger(2.);
    while task.is_ongoing() {}
    let event: Option<f64> = dbg!(task.event());
    assert_eq!(event, Some(3.));
}
