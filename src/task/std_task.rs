use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use super::Function;

pub(super) struct TaskStd<F: Function> {
    input: Sender<F::Input>,
    output: Receiver<F::Output>,
    _thread: JoinHandle<()>,
}
impl<F: Function> TaskStd<F> {
    pub(super) fn new(thread_name: &str) -> Self {
        let (input_sender, input_receiver) = channel();
        let (output_sender, output_receiver) = channel();
        let thread = std::thread::Builder::new()
            .name(thread_name.into())
            .spawn(move || {
                let mut function = F::default();
                while let Ok(input) = input_receiver.recv() {
                    let output = function.call(input);
                    if output_sender.send(output).is_err() {
                        break;
                    }
                }
            })
            .unwrap();
        Self {
            input: input_sender,
            output: output_receiver,
            _thread: thread,
        }
    }
    pub(super) fn enqueue(&self, msg: F::Input) {
        let r = self.input.send(msg);
        assert!(r.is_ok());
    }
    pub(super) fn check(&self) -> Option<F::Output> {
        self.output.try_recv().ok()
    }
}

#[test]
fn test_task_std() {
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

    let task = TaskStd::<DummyFunction>::new("dummy_thread");
    let n = 10;
    for i in 0..n {
        task.enqueue(i);
    }
    for i in 0..n {
        let i = (i + 1) as u64;
        let v = loop {
            match task.check() {
                Some(v) => break v,
                None => continue,
            }
        };
        assert_eq!(i, v);
    }
}
