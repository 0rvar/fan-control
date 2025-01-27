use std::thread::JoinHandle;

use esp_idf_hal::cpu::Core;

pub struct EspThread {
    name: &'static str,
    stack_kb: usize,
    priority: u8,
    pin_to_core: Option<Core>,
}
impl EspThread {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            stack_kb: 4,
            priority: 10,
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

        // ThreadSpawnConfiguration does _nothing_
        // ThreadSpawnConfiguration {
        //     name: Some(name),
        //     stack_size: stack_kb * 1024,
        //     priority,
        //     pin_to_core,
        //     ..Default::default()
        // }
        // .set()
        // .unwrap();

        let handle = std::thread::Builder::new()
            .name(name.to_string())
            .stack_size(stack_kb * 1024)
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));
                if let Err(err) = result {
                    log::error!("Thread panicked: {:?}", err);
                }
            })
            .unwrap();

        // ThreadSpawnConfiguration::default().set().unwrap();

        handle
    }
}

pub fn debug_dump_stack_info() {
    let task_name =
        unsafe { std::ffi::CStr::from_ptr(esp_idf_hal::sys::pcTaskGetName(std::ptr::null_mut())) }
            .to_string_lossy();

    let free_stack = unsafe { esp_idf_hal::sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()) };
    log::info!("[{task_name}] Free stack: {free_stack}b");

    let free_heap = unsafe { esp_idf_hal::sys::xPortGetFreeHeapSize() };
    log::info!("[{task_name}] Free heap: {free_heap}b");
}
