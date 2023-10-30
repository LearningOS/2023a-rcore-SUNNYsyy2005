//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use crate::task::TaskStatus;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
const  BIG_STRIDE:usize=10000;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
       // self.ready_queue.pop_front()
         let mut min_stride = usize::MAX;
        let mut next_task = None;
        let mut val =  VecDeque::new();
        while let Some(task) = self.ready_queue.pop_front() {
            
            let task_inner = task.inner_exclusive_access();
           // print!("get pid:{}\n",task.getpid());
            if task_inner.task_status == TaskStatus::Ready {
                let stride = task_inner.stride;
                if stride < min_stride {
                    min_stride = stride;
                    next_task = Some(task.clone());
                }
            }
            drop(task_inner);
            val.push_back(task.clone());
        };
        if let Some(task) = &next_task {
            let mut task_inner = task.inner_exclusive_access();
            task_inner.stride += BIG_STRIDE / task_inner.priority;
            drop(task_inner);
        }
        while let Some(task) = val.pop_front() {
            if task.getpid() != next_task.clone().unwrap().getpid(){
             //   print!("push back:{} \n",task.getpid());
                self.ready_queue.push_back(task.clone());
            }else{
              //  print!("will exec new pid:{} \n",task.getpid());
            }
        }
        next_task 
    }
}


lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    trace!("kernel: TaskManager::fetch_task");
   // print!("fetch task\n");
    TASK_MANAGER.exclusive_access().fetch()
}
