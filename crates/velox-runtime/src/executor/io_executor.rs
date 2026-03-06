use std::future::Future;
use std::io;

pub struct IoExecutor {
    runtime: tokio::runtime::Runtime,
}

impl IoExecutor {
    pub fn new() -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self { runtime })
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }
}
