use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

// ─── Priority Task Types ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Idle = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub priority: TaskPriority,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TaskScheduler {
    queue: VecDeque<Task>,
    active_workers: usize,
    max_workers: usize,
    next_task_id: u64,
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(4)
    }
}

impl TaskScheduler {
    /// Initialize a task scheduler up to `max_workers` limit.
    pub fn new(max_workers: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            active_workers: 0,
            max_workers,
            next_task_id: 1,
        }
    }

    /// Submit a task, returning its unique sequential task ID.
    pub fn submit(&mut self, name: String, priority: TaskPriority) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;

        self.queue.push_back(Task { id, priority, name });

        id
    }

    /// Retrieves and removes the highest priority task available.
    pub fn poll(&mut self) -> Option<Task> {
        if self.queue.is_empty() {
            return None;
        }

        let mut best_idx = 0;
        let mut best_priority = self.queue[0].priority;

        for (i, task) in self.queue.iter().enumerate().skip(1) {
            if task.priority > best_priority {
                best_idx = i;
                best_priority = task.priority;
            }
        }

        self.queue.remove(best_idx)
    }

    /// Adjust active worker count dynamically to save power based on complexity.
    pub fn adapt_to_workload(&mut self, scene_node_count: usize) {
        self.active_workers = if scene_node_count <= 50 {
            1
        } else if scene_node_count <= 500 {
            2
        } else if scene_node_count <= 2000 {
            4
        } else {
            self.max_workers
        };

        if self.active_workers > self.max_workers {
            self.active_workers = self.max_workers;
        }
    }

    /// Current number of dynamically scaled active workers.
    pub fn active_workers(&self) -> usize {
        self.active_workers
    }

    /// Number of incomplete tasks sitting in the scheduler queue.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

// ─── Concurrent Threaded Pipeline Scheduler ───────────────────────

/// Typed pipeline stages that can be dispatched asynchronously to worker threads.
#[derive(Debug)]
pub enum PipelineStage {
    /// Load HTML or CSS resources from disk/network
    LoadResource { url: String },
    /// Tokenize and parse HTML bytes into a DOM tree
    ParseHtml { url: String, bytes: Vec<u8> },
    /// Tokenize and parse CSS bytes into a Stylesheet
    ParseCss { url: String, bytes: Vec<u8> },
}

/// Results returned by completed pipeline tasks.
#[derive(Debug)]
pub enum TaskResult {
    /// Resource loaded via ResourceLoader
    ResourceLoaded(crate::loader::PageResources),
    /// Parsed HTML DOM tree and total token count
    HtmlParsed {
        url: String,
        dom: crate::dom::Dom,
        tokens_count: usize,
    },
    /// Parsed CSS Stylesheet and rule count
    CssParsed {
        url: String,
        stylesheet: crate::css_parser::Stylesheet,
    },
}

/// Message payload returned over the channel from worker threads.
pub struct TaskMessage {
    pub task_id: u64,
    pub result: Result<TaskResult, String>,
}

/// Job envelope sent to worker threads.
struct WorkerJob {
    task_id: u64,
    stage: PipelineStage,
}

/// Multi-threaded task scheduler managing background worker threads and `mpsc` channel queues.
pub struct ThreadedScheduler {
    next_id: u64,
    job_sender: Option<mpsc::Sender<WorkerJob>>,
    result_receiver: mpsc::Receiver<TaskMessage>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl ThreadedScheduler {
    /// Create a new `ThreadedScheduler` spawning `num_workers` background threads.
    pub fn new(num_workers: usize) -> Self {
        let (job_sender, job_receiver) = mpsc::channel::<WorkerJob>();
        let (result_sender, result_receiver) = mpsc::channel::<TaskMessage>();

        let job_receiver = Arc::new(Mutex::new(job_receiver));
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let receiver = Arc::clone(&job_receiver);
            let tx = result_sender.clone();

            let handle = thread::spawn(move || {
                loop {
                    let job = {
                        let rx = receiver.lock().unwrap();
                        rx.recv()
                    };

                    match job {
                        Ok(WorkerJob { task_id, stage }) => {
                            let stage_res =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    execute_stage(stage)
                                }));

                            let res = match stage_res {
                                Ok(res) => res,
                                Err(panic_payload) => {
                                    let msg = if let Some(s) = panic_payload.downcast_ref::<&str>()
                                    {
                                        s.to_string()
                                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                        s.clone()
                                    } else {
                                        "Task panicked during execution".to_string()
                                    };
                                    Err(format!("Task panic: {}", msg))
                                }
                            };

                            if tx
                                .send(TaskMessage {
                                    task_id,
                                    result: res,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break, // Channel closed — shutdown thread
                    }
                }
            });

            workers.push(handle);
        }

        ThreadedScheduler {
            next_id: 1,
            job_sender: Some(job_sender),
            result_receiver,
            workers,
        }
    }

    /// Schedule a pipeline stage for background processing.
    /// Returns the unique `u64` task ID.
    pub fn schedule(&mut self, stage: PipelineStage) -> Result<u64, String> {
        let task_id = self.next_id;
        self.next_id += 1;

        if let Some(ref sender) = self.job_sender {
            sender
                .send(WorkerJob { task_id, stage })
                .map_err(|e| format!("Failed to dispatch task: {}", e))?;
            Ok(task_id)
        } else {
            Err("Scheduler is shut down".to_string())
        }
    }

    /// Non-blocking poll to receive completed task results from worker threads.
    pub fn poll_completed(&self) -> Option<TaskMessage> {
        self.result_receiver.try_recv().ok()
    }

    /// Blocking wait for the next completed task result.
    pub fn recv_completed(&self) -> Option<TaskMessage> {
        self.result_receiver.recv().ok()
    }

    /// Cleanly shut down worker threads.
    pub fn shutdown(&mut self) {
        self.job_sender.take(); // Drop sender to close worker channel
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for ThreadedScheduler {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Execute a pipeline stage on a background thread.
fn execute_stage(stage: PipelineStage) -> Result<TaskResult, String> {
    match stage {
        PipelineStage::LoadResource { url } => {
            let mut loader = crate::loader::ResourceLoader::new();
            if url == "<sample>" {
                let resources = loader.load_html_string(
                    "<!DOCTYPE html><html><body><h1>Sample</h1></body></html>",
                    "<sample>",
                );
                Ok(TaskResult::ResourceLoaded(resources))
            } else {
                loader
                    .load_file(&url)
                    .map(TaskResult::ResourceLoaded)
                    .map_err(|e| e.to_string())
            }
        }
        PipelineStage::ParseHtml { url, bytes } => {
            let mut tokenizer = crate::tokenizer::Tokenizer::new(&bytes);
            let tokens = tokenizer.tokenize();
            let tokens_count = tokens.len();
            let parser = crate::parser::Parser::new(&tokens, &bytes);
            let dom = parser.parse();
            Ok(TaskResult::HtmlParsed {
                url,
                dom,
                tokens_count,
            })
        }
        PipelineStage::ParseCss { url, bytes } => {
            let stylesheet = crate::css_parser::Stylesheet::parse(&bytes);
            Ok(TaskResult::CssParsed { url, stylesheet })
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_scheduler() {
        let mut sched = TaskScheduler::new(4);
        let id1 = sched.submit("Low task".to_string(), TaskPriority::Low);
        let id2 = sched.submit("Critical task".to_string(), TaskPriority::Critical);
        let id3 = sched.submit("Normal task".to_string(), TaskPriority::Normal);

        assert_eq!(sched.pending_count(), 3);

        // Highest priority should be polled first
        let polled1 = sched.poll().unwrap();
        assert_eq!(polled1.id, id2);
        assert_eq!(polled1.priority, TaskPriority::Critical);

        let polled2 = sched.poll().unwrap();
        assert_eq!(polled2.id, id3);

        let polled3 = sched.poll().unwrap();
        assert_eq!(polled3.id, id1);

        assert!(sched.poll().is_none());
    }

    #[test]
    fn test_adapt_workload() {
        let mut sched = TaskScheduler::new(8);
        sched.adapt_to_workload(30);
        assert_eq!(sched.active_workers(), 1);

        sched.adapt_to_workload(300);
        assert_eq!(sched.active_workers(), 2);

        sched.adapt_to_workload(1500);
        assert_eq!(sched.active_workers(), 4);

        sched.adapt_to_workload(5000);
        assert_eq!(sched.active_workers(), 8);
    }

    #[test]
    fn test_threaded_scheduler_parse_html() {
        let mut scheduler = ThreadedScheduler::new(2);
        let html = b"<html><body><h1>Async Parse</h1></body></html>".to_vec();
        let task_id = scheduler
            .schedule(PipelineStage::ParseHtml {
                url: "test.html".to_string(),
                bytes: html,
            })
            .expect("Schedule failed");

        let msg = scheduler.recv_completed().expect("Recv failed");
        assert_eq!(msg.task_id, task_id);

        match msg.result {
            Ok(TaskResult::HtmlParsed {
                url,
                dom,
                tokens_count,
            }) => {
                assert_eq!(url, "test.html");
                assert!(dom.nodes.len() > 1);
                assert!(tokens_count > 0);
            }
            _ => panic!("Expected HtmlParsed result"),
        }
    }

    #[test]
    fn test_threaded_scheduler_parse_css() {
        let mut scheduler = ThreadedScheduler::new(2);
        let css = b"h1 { color: red; } p { margin: 10px; }".to_vec();
        let task_id = scheduler
            .schedule(PipelineStage::ParseCss {
                url: "style.css".to_string(),
                bytes: css,
            })
            .expect("Schedule failed");

        let msg = scheduler.recv_completed().expect("Recv failed");
        assert_eq!(msg.task_id, task_id);

        match msg.result {
            Ok(TaskResult::CssParsed { url, stylesheet }) => {
                assert_eq!(url, "style.css");
                assert_eq!(stylesheet.rules.len(), 2);
            }
            _ => panic!("Expected CssParsed result"),
        }
    }

    #[test]
    fn test_threaded_scheduler_load_resource_sample() {
        let mut scheduler = ThreadedScheduler::new(2);
        let task_id = scheduler
            .schedule(PipelineStage::LoadResource {
                url: "<sample>".to_string(),
            })
            .expect("Schedule failed");

        let msg = scheduler.recv_completed().expect("Recv failed");
        assert_eq!(msg.task_id, task_id);

        match msg.result {
            Ok(TaskResult::ResourceLoaded(page)) => {
                assert_eq!(page.html.url, "<sample>");
            }
            _ => panic!("Expected ResourceLoaded result"),
        }
    }

    #[test]
    fn test_threaded_scheduler_panic_isolation() {
        let mut scheduler = ThreadedScheduler::new(2);
        let non_existent_task = scheduler
            .schedule(PipelineStage::LoadResource {
                url: "non_existent_file_xyz_12345.html".to_string(),
            })
            .expect("Schedule failed");

        let msg = scheduler.recv_completed().expect("Recv failed");
        assert_eq!(msg.task_id, non_existent_task);
        assert!(msg.result.is_err());

        // Worker thread survived and can process next task
        let html = b"<html><body><p>OK</p></body></html>".to_vec();
        let valid_task = scheduler
            .schedule(PipelineStage::ParseHtml {
                url: "valid.html".to_string(),
                bytes: html,
            })
            .expect("Schedule failed");

        let msg2 = scheduler.recv_completed().expect("Recv failed");
        assert_eq!(msg2.task_id, valid_task);
        assert!(msg2.result.is_ok());
    }
}
