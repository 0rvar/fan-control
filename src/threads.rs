use std::thread::JoinHandle;

use esp_idf_hal::{cpu::Core, task::thread::ThreadSpawnConfiguration};

pub struct EspThread {
    name: &'static [u8],
    stack_kb: usize,
    priority: u8,
    pin_to_core: Option<Core>,
}
impl EspThread {
    pub fn new(name: &'static [u8]) -> Self {
        Self {
            name,
            stack_kb: 4,
            priority: 5,
            pin_to_core: None,
        }
    }
    pub fn with_stack_size(mut self, stack_kb: usize) -> Self {
        self.stack_kb = stack_kb;
        self
    }
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
    pub fn pin_to_core(mut self, core: Core) -> Self {
        self.pin_to_core = Some(core);
        self
    }
    pub fn spawn<F>(self, func: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        let Self {
            name,
            stack_kb,
            priority,
            pin_to_core,
        } = self;
        ThreadSpawnConfiguration {
            name: Some(name),
            stack_size: stack_kb * 1024,
            priority,
            pin_to_core,
            ..Default::default()
        }
        .set()
        .unwrap();

        let handle = std::thread::Builder::new().spawn(func).unwrap();

        ThreadSpawnConfiguration::default().set().unwrap();

        handle
    }
}
