//! Process management syscalls



use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token, TASK_MANAGER,
    }, mm::{VirtPageNum,  translated_byte_buffer,  VirtAddr, MapPermission}, timer::get_time_us, syscall::{SYSCALL_GET_TIME, SYSCALL_YIELD, SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_TASK_INFO},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    addinfo(SYSCALL_EXIT);
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    addinfo(SYSCALL_YIELD);
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    addinfo(SYSCALL_GET_TIME);
    let len = core::mem::size_of::<TimeVal>();
    let mut buffers = translated_byte_buffer(current_user_token(), ts as *const u8, len);
    let us = get_time_us();
    
    let time = TimeVal{
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    // 将time的数据复制到buffers中存储的地址
    unsafe {
        core::ptr::copy_nonoverlapping(&time, buffers[0].as_mut_ptr() as *mut TimeVal, 1);
    }
    0
}

pub fn addinfo(tt:usize) -> isize {
    trace!("kernel: addinfo");
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    match inner.tasks.get_mut(current) {
        Some(current_task) => {
            match tt {
                SYSCALL_EXIT=>{current_task.exit_count+=1;},
                SYSCALL_YIELD=>{current_task.yield_count+=1},
                SYSCALL_GET_TIME=>{current_task.gettime_count+=1},
                SYSCALL_WRITE=>{current_task.write_count+=1},
                _=>{},
            }
        },
        None => {
            // 处理 inner.tasks.get_mut(current) 返回 None 的情况
        },
    }
    0
}
/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let len = core::mem::size_of::<TaskInfo>();
    let mut buffers = translated_byte_buffer(current_user_token(), ti as *const u8, len);
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let mut syscall_times1 = [0; MAX_SYSCALL_NUM];
    match inner.tasks.get_mut(current) {
        Some(current_task) => {
            current_task.taskinfo_count+=1;
            syscall_times1[SYSCALL_GET_TIME] = current_task.gettime_count;
            syscall_times1[SYSCALL_YIELD] = current_task.yield_count;
            syscall_times1[SYSCALL_WRITE] = current_task.write_count;
            syscall_times1[SYSCALL_EXIT] = current_task.exit_count;
            syscall_times1[SYSCALL_TASK_INFO] = current_task.taskinfo_count;
            // 这里可以使用 current_task 的可变引用
        },
        None => {
            // 处理 inner.tasks.get_mut(current) 返回 None 的情况
        },
    }
    let taskinfo=TaskInfo {
        status: inner.tasks[current].task_status,
        time: get_time_us()/1000,
        syscall_times:syscall_times1,
    };
    unsafe {
        core::ptr::copy_nonoverlapping(&taskinfo, buffers[0].as_mut_ptr() as *mut TaskInfo, 1);
    }
    0
}
static  mut TOTAL:i32=0;
// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    let page_size = 4096;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);
    print!("start:{},page_align_start:{},page_align_len:{}\n",start,page_align_start,page_align_len);
    if start != page_align_start {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let page_table = &mut inner.tasks[current].user_memory_set;
    let mut flag=0;
    for i in 0..(page_align_len / page_size) {
        unsafe{
            TOTAL+=1;
            print!("total:{}\n",TOTAL);
        }
        let vaddr = page_align_start + i * page_size;
        let t = page_table.translate(VirtPageNum::from(VirtAddr::from(vaddr)));
        match t {
            Some(_) => {
                print!("error\n");
                flag = -1;
            }
            None => {
                print!("{}\n ",vaddr);
                let portz=1<<4+port;
                page_table.insert_framed_area(
                    VirtAddr::from(vaddr) ,
                    VirtAddr::from(vaddr+page_size),
                    MapPermission::from_bits_truncate(portz as u8),
                );
                let tt=page_table.translate(VirtPageNum::from(VirtAddr::from(vaddr)));
                match tt {
                    Some(_)=>{
                        print!("Success");
                    }
                    None=>{
                        print!("Error");
                    }
                }
            }
        }
    } 
    flag
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let page_size = 512;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);

    if start != page_align_start {
        return -1;
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let page_table = &mut inner.tasks[current].user_memory_set;
    let mut flag=-1;
    for j in 0..(page_align_len / page_size) {
        let vaddr = page_align_start + j * page_size;

        let t = page_table.translate(VirtPageNum(vaddr));
        match t {
            Some(_) => {
                page_table.delete_framed_area(vaddr);
                flag=0;
            }
            None => {
                
            }
            
        }
    }
    flag
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
