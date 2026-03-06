use std::future::Future;

pub struct IoExecutor {
    runtime: tokio::runtime::Runtime,
}

impl IoExecutor {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("velox-io")
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for IoExecutor");
        Self { runtime }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }

    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }
}

impl Default for IoExecutor {
    fn default() -> Self {
        Self::new()
    }
}
