use std::thread::JoinHandle;

pub struct EspThread {
    name: &'static str,
    stack_kb: Option<usize>,
}
impl EspThread {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            stack_kb: None,
        }
    }
    pub fn with_stack_size(mut self, stack_kb: usize) -> Self {
        self.stack_kb = Some(stack_kb);
        self
    }
    pub fn spawn<F>(self, func: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        let Self { name, stack_kb } = self;

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

        let mut builder = std::thread::Builder::new().name(name.to_string());

        if let Some(stack_kb) = stack_kb {
            builder = builder.stack_size(stack_kb * 1024);
        }

        let handle = builder
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
