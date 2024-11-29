use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct ThreadPool {
    task_sender: mpsc::Sender<Task>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (task_sender, task_receiver) = mpsc::channel::<Task>(200);
        let task_receiver = Arc::new(Mutex::new(task_receiver));

        for _ in 0..size {
            let task_receiver = task_receiver.clone();

            tokio::spawn(async move {
                loop {
                    let task = {
                        let mut receiver = task_receiver.lock().await;
                        receiver.recv().await
                    };

                    match task {
                        Some(task) => {
                            // info!("Worker {} processing task", id);
                            task.await;
                        }
                        None => break,
                    }
                }
            });
        }

        ThreadPool { task_sender }
    }

    pub async fn execute<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let _ = self.task_sender.send(Box::pin(f)).await;
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new(25)
    }
}
