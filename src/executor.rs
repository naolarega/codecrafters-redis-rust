use std::{
    fs::File,
    io::Read,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

struct ThreadWrapper {
    thread_number: usize,
    occupied: Arc<Mutex<bool>>,
    task_added: Arc<(Mutex<bool>, Condvar)>,
    task_queue: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl ThreadWrapper {
    fn new(thread_number: usize) -> Self {
        Self {
            thread_number,
            occupied: Arc::new(Mutex::new(false)),
            task_added: Arc::new((Mutex::new(false), Condvar::new())),
            task_queue: Arc::new(Mutex::new(Vec::new())),
            thread_join_handle: None,
        }
    }

    fn start(&mut self) {
        let occupied = Arc::clone(&self.occupied);
        let task_added = Arc::clone(&self.task_added);
        let task_queue = Arc::clone(&self.task_queue);

        self.thread_join_handle = Some(
            thread::Builder::new()
                .name(format!("Redis thread {}", self.thread_number))
                .spawn(move || loop {
                    if (&*task_queue).lock().unwrap().is_empty() {
                        let (mtx, cvar) = &*task_added;
                        let added = mtx.lock().unwrap();

                        cvar.wait(added).unwrap();
                    }

                    if let Some(task) = (&*task_queue).lock().unwrap().pop() {
                        *(&*occupied).lock().unwrap() = true;

                        task();
                    }

                    if (&*task_queue).lock().unwrap().is_empty() {
                        *(&*occupied).lock().unwrap() = false;
                    }
                })
                .unwrap(),
        );
    }

    fn is_occupied(&self) -> bool {
        *(self.occupied.lock().unwrap())
    }

    fn is_live(&self) -> bool {
        if let Some(ref thread_join_handle) = self.thread_join_handle {
            if !thread_join_handle.is_finished() {
                return true;
            }
        }

        false
    }

    fn task_queue_size(&self) -> usize {
        self.task_queue.lock().unwrap().len()
    }

    fn set_task<T>(&mut self, task: T)
    where
        T: FnOnce() + Send + 'static,
    {
        if let Ok(mut task_queue) = self.task_queue.lock() {
            (*task_queue).push(Box::new(task));

            let (mtx, cvar) = &*self.task_added;
            let mut added = mtx.lock().unwrap();

            *added = true;

            cvar.notify_one();
        }
    }
}

pub struct ThreadPoolExecutor {
    thread_count: usize,
    threads: Vec<ThreadWrapper>,
    dead_thread_numbers: Vec<usize>,
}

impl ThreadPoolExecutor {
    pub fn new() -> Self {
        Self {
            thread_count: Self::cpu_count(),
            threads: Vec::new(),
            dead_thread_numbers: Vec::new(),
        }
    }

    pub fn submit<T>(&mut self, task: T)
    where
        T: FnOnce() + Send + 'static,
    {
        self.evict_dead_threads();

        let free_thread = self
            .threads
            .iter_mut()
            .find(|thread_wrapper| thread_wrapper.is_live() && !thread_wrapper.is_occupied());

        if let Some(free_thread) = free_thread {
            free_thread.set_task(task);
        } else {
            if self.threads.len() < self.thread_count {
                let thread_number;

                if self.dead_thread_numbers.is_empty() {
                    thread_number = self.threads.len()
                } else {
                    thread_number = self.dead_thread_numbers.first().unwrap().to_owned();
                }

                let mut thread_wrapper = ThreadWrapper::new(thread_number);

                thread_wrapper.start();
                thread_wrapper.set_task(task);

                self.threads.push(thread_wrapper);
            } else {
                if let Some(least_occupied_thread) = self
                    .threads
                    .iter_mut()
                    .min_by(|a, b| a.task_queue_size().cmp(&b.task_queue_size()))
                {
                    least_occupied_thread.set_task(task);
                } else {
                    self.threads.first_mut().unwrap().set_task(task);
                }
            }
        }
    }

    fn evict_dead_threads(&mut self) {
        self.dead_thread_numbers
            .extend(self.threads.iter().filter_map(|thread_wrapper| {
                if !thread_wrapper.is_live() {
                    Some(thread_wrapper.thread_number)
                } else {
                    None
                }
            }));

        for thread_number in self.dead_thread_numbers.iter() {
            if let Some(index) =
                self.threads
                    .iter()
                    .enumerate()
                    .find_map(|(index, thread_wrapper)| {
                        if thread_wrapper.thread_number == *thread_number {
                            Some(index)
                        } else {
                            None
                        }
                    })
            {
                self.threads.remove(index);
            }
        }
    }

    fn cpu_count() -> usize {
        let mut cpuinfo = String::new();

        File::open("/proc/cpuinfo")
            .unwrap()
            .read_to_string(&mut cpuinfo)
            .unwrap();

        cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count()
    }
}
