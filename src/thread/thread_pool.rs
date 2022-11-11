use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

enum Message {
    NewJob(Job),
    Terminate,
}

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    job_sender: mpsc::Sender<Message>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// 创建线程池。
    ///
    /// `size`线程池中线程的数量。
    ///
    /// # Panics
    ///
    /// `new` 函数在 size 为 0 时会 panic
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        // 控制线程
        let (job_sender, job_receiver) = mpsc::channel();

        let job_receiver = Arc::new(Mutex::new(job_receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&job_receiver)));
        }

        ThreadPool {
            workers,
            job_sender,
        }
    }

    // 需要在多线程中执行的闭包函数
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.job_sender.send(Message::NewJob(job)).unwrap();
    }
}

/// 工作任务
#[derive(Debug)]
struct Worker {
    /// 工作线程id
    id: usize,
    /// 保存创建的线程
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            // 锁定接受者对象用于获取数据，尝试等待此接收者上的值阻塞当前线程，自动分配线程池的核心功能
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                // 工作消息执行工作
                Message::NewJob(job) => {
                    job();
                }
                // 关闭线程消息
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
    // 在清理数据时结束线程
    fn drop(&mut self) {
        for _ in &self.workers {
            self.job_sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
