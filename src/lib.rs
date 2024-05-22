#![warn(
    anonymous_parameters,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

//! This is a basic implementation of a task which can be used but with Standard (Linux,Windows, MacOs) and Wasm (Web).
//! A task means here a function which can be run in the background (Standard:Thread, Wasm: WebWorker).

#[cfg(not(target_arch = "wasm32"))]
mod std_task;
mod ticket;
#[cfg(target_arch = "wasm32")]
mod wasm_task;

#[cfg(target_arch = "wasm32")]
pub use gloo_worker;
use std::collections::{HashMap, VecDeque};
pub use ticket::Ticket;
#[cfg(target_arch = "wasm32")]
pub use wasm_task::WebWorker;

/// This trait abstracts a function, which can be run independently
pub trait Function: 'static + Default + Sized {
    /// Input type of function
    type Input: serde::Serialize + serde::de::DeserializeOwned + Send;
    /// Output type of function
    type Output: serde::Serialize + serde::de::DeserializeOwned + Send;
    /// Function to run
    fn call(&mut self, input: Self::Input) -> Self::Output;
}

/// This is a single task
pub struct Task<F: Function> {
    #[cfg(not(target_arch = "wasm32"))]
    task: std_task::TaskStd<F>,
    #[cfg(target_arch = "wasm32")]
    task: wasm_task::TaskWasm<F>,
}
impl<F: Function> std::fmt::Debug for Task<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").finish()
    }
}

impl<F: Function> Task<F> {
    /// Start a new task in the background. Enqueue jobs to run in the background.
    pub fn new(task_name: &str) -> Self {
        Self {
            task: {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    std_task::TaskStd::new(task_name)
                }
                #[cfg(target_arch = "wasm32")]
                {
                    wasm_task::TaskWasm::new(task_name)
                }
            },
        }
    }
    /// Enqueue a new job.
    pub fn enqueue(&mut self, msg: F::Input) {
        self.task.enqueue(msg);
    }
    /// Check if the job is done (using First In, First Out)
    pub fn check(&self) -> Option<F::Output> {
        self.task.check()
    }
}

/// Task Pool which can run several jobs in parallel.
#[derive(Debug)]
pub struct TaskPool<F: Function> {
    tasks: Vec<(Option<Ticket>, Task<F>)>,
    to_start: VecDeque<(Ticket, F::Input)>,
    done: HashMap<Ticket, F::Output>,
    ticket_generator: ticket::TicketGenerator,
}
impl<F: Function> TaskPool<F> {
    /// Create a new TaskPool.
    #[must_use]
    pub fn new(task_name: &str, task_count: usize) -> Self {
        Self {
            tasks: (0..task_count)
                .map(|_| (None, Task::new(task_name)))
                .collect(),
            to_start: Default::default(),
            done: Default::default(),
            ticket_generator: Default::default(),
        }
    }
    /// Progress all enqueued jobs.
    pub fn progress(&mut self) {
        for (ongoing, task) in self.tasks.iter_mut() {
            if ongoing.is_some() {
                if let Some(output) = task.check() {
                    let ticket = std::mem::take(ongoing).unwrap();
                    let r = self.done.insert(ticket, output);
                    if r.is_some() {
                        panic!("Ticket is already in list of done jobs")
                    }
                }
            }
            if ongoing.is_none() {
                if let Some((ticket, input)) = self.to_start.pop_front() {
                    *ongoing = Some(ticket);
                    task.enqueue(input);
                }
            }
        }
    }
    /// Enqueue a new job. Use the returned ticket to check later if the job is done.
    #[must_use]
    pub fn enqueue(&mut self, input: F::Input) -> Ticket {
        let (ticket, ticket_internal) = self.ticket_generator.next();
        self.to_start.push_back((ticket_internal, input));
        self.progress();
        ticket
    }
    /// Check if a job is done.
    #[must_use]
    pub fn check(&mut self, ticket: Ticket) -> JobState<F::Output> {
        self.progress();
        if let Some(output) = self.done.remove(&ticket) {
            JobState::Done(output)
        } else {
            JobState::Ongoing(ticket)
        }
    }

    /// Wait for a job to finish
    #[must_use]
    pub fn wait_for(&mut self, ticket: Ticket) -> F::Output {
        match self.check(ticket) {
            JobState::Ongoing(ticket) => self.wait_for(ticket),
            JobState::Done(output) => output,
        }
    }
}

/// This is the current state of a job.
#[derive(Debug)]
pub enum JobState<Output> {
    /// The job is not yet done. Use this ticket to check later.
    Ongoing(Ticket),
    /// The job is done, yielding output.
    Done(Output),
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_task_pool_std() {
    #[derive(Default)]
    struct DummyFunction;
    impl Function for DummyFunction {
        type Input = u32;
        type Output = u64;

        fn call(&mut self, input: Self::Input) -> Self::Output {
            doubling(input)
        }
    }
    fn doubling(x: u32) -> u64 {
        (x + 1) as _
    }

    let mut task_pool = TaskPool::<DummyFunction>::new("dummy_thread", 3);
    let n = 10;
    let mut tickets = Vec::new();
    for i in 0..n {
        tickets.push(task_pool.enqueue(i));
    }
    for (i, ticket) in tickets.into_iter().enumerate() {
        let i = (i + 1) as u64;
        let v = task_pool.wait_for(ticket);
        assert_eq!(i, v);
    }
}
