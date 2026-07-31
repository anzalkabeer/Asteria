use std::collections::VecDeque;

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
        
        self.queue.push_back(Task {
            id,
            priority,
            name,
        });
        
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
