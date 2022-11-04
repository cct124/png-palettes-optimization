use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

enum Message {
    NewJob(Job, usize),
    Terminate,
}

#[derive(Debug)]
pub enum WorkStatus {
    INIT,
    End,
    WAIT,
}

#[derive(Debug)]
pub struct Status {
    pub id: usize,
    pub status: WorkStatus,
}

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    job_sender: mpsc::Sender<Message>,
    pub status_receiver: mpsc::Receiver<Status>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// 创建线程池。
    ///
    /// 线程池中线程的数量。
    ///
    /// # Panics
    ///
    /// `new` 函数在 size 为 0 时会 panic
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (job_sender, job_receiver) = mpsc::channel();
        let (status_sender, status_receiver) = mpsc::channel();

        let job_receiver = Arc::new(Mutex::new(job_receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(
                id,
                Arc::clone(&job_receiver),
                status_sender.clone(),
            ));
        }

        ThreadPool {
            workers,
            job_sender,
            status_receiver,
        }
    }

    pub fn execute<F>(&self, f: F, work_id: usize)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.job_sender.send(Message::NewJob(job, work_id)).unwrap();
    }
}

#[derive(Debug)]
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
        status_sender: mpsc::Sender<Status>,
    ) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job, work_id) => {
                    job();
                    status_sender
                        .send(Status {
                            id: work_id,
                            status: WorkStatus::End,
                        })
                        .unwrap();
                }

                Message::Terminate => break,
            }
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.job_sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
