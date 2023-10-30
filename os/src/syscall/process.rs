//! Process management syscalls


use alloc::sync::Arc;


use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str, translated_byte_buffer, VirtPageNum, VirtAddr, MapPermission},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, 
    }, syscall::{SYSCALL_YIELD, SYSCALL_GET_TIME, SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_TASK_INFO}, timer::get_time_us,

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
pub fn sys_exit(exit_code: i32) -> ! {
    addinfo(SYSCALL_EXIT);
   // print!("sys_exit\n");
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    addinfo(SYSCALL_YIELD);
   // print!("sys_yield\n");
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
   // print!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
   // print!("sys_yield_success\n");
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
   // print!("sys_fork\n");
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
   // print!("sys_fork success new_pid:{}\n",new_pid);

    new_pid as isize
}
pub fn addinfo(tt:usize) -> isize {
    trace!("kernel: addinfo");
    {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    match tt {
        SYSCALL_EXIT=>{inner.exit_count+=1;},
        SYSCALL_YIELD=>{inner.yield_count+=1},
        SYSCALL_GET_TIME=>{inner.gettime_count+=1},
        SYSCALL_WRITE=>{inner.write_count+=1},
        _=>{},
    };
    } 
    0
}
pub fn sys_exec(path: *const u8) -> isize {
 //   trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
   // print!("sys_exec\n");
    addinfo(SYSCALL_EXIT);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
  //  print!("sys_waitpid begin\n");
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    //print!("watch pid\n");
    let task = current_task().unwrap();
    // find a child process
    
    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
      //  print!("sys_waitpid end\n");
        found_pid as isize
    } else {
        -2
    }
    
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",0
      //  current_task().unwrap().pid.0
    );
  //  print!("getting time!\n");
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

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",0
       // current_task().unwrap().pid.0
    );
   // print!("getting info!\n");
    let len = core::mem::size_of::<TaskInfo>();
    let mut buffers = translated_byte_buffer(current_user_token(), ti as *const u8, len);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let mut syscall_times1 = [0; MAX_SYSCALL_NUM];
        inner.taskinfo_count+=1;
        
        syscall_times1[SYSCALL_GET_TIME] = inner.gettime_count;
        syscall_times1[SYSCALL_YIELD] = inner.yield_count;
        syscall_times1[SYSCALL_WRITE] = inner.write_count;
        syscall_times1[SYSCALL_EXIT] = inner.exit_count;
        syscall_times1[SYSCALL_TASK_INFO] = inner.taskinfo_count;
        
    let taskinfo=TaskInfo {
        status: inner.task_status,
        time: get_time_us()/1000,
        syscall_times:syscall_times1,
    };//print!("taskinfo_count now:{}\n",taskinfo.syscall_times[SYSCALL_TASK_INFO]);
    unsafe {
        core::ptr::copy_nonoverlapping(&taskinfo, buffers[0].as_mut_ptr() as *mut TaskInfo, 1);
    }
    0
}
/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",0
      //  current_task().unwrap().pid.0
    );
  //  print!("map\n");
    let page_size = 4096;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);
    if start != page_align_start {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let mut permission = MapPermission::U;
    if port & 1 !=0 {permission = permission | MapPermission::R;print!("readable\n");}
    if port & 2 !=0 {permission = permission | MapPermission::W;print!("writeable\n");}
    if port & 4 !=0 {permission = permission | MapPermission::X;print!("exeable\n");}
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let page_table = &mut inner.memory_set;
    let mut flag=0;
    for i in 0..(page_align_len / page_size){
        let vaddr = page_align_start + i * page_size;
        let start_vpn: VirtPageNum = VirtAddr::from(vaddr).floor();
        let t = page_table.translate(start_vpn);
        if t.is_some(){
            if t.unwrap().is_valid(){
                flag=-1;
            }
            
        }
       // let end_vpn:VirtPageNum=VirtAddr::from(vaddr+page_size).ceil();
        if flag==0{
         //   print!("insert {:x}\n",vaddr);
            page_table.insert_framed_area(
            VirtAddr::from(vaddr),
            VirtAddr::from(vaddr+page_size-1),
                permission,
            );
        }
    }
    flag
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",0
     //   current_task().unwrap().pid.0
    );
   // print!("unmap\n");
    let page_size = 4096;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);

    if start != page_align_start {
        return -1;
    }
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let page_table = &mut inner.memory_set;
    let mut flag=0;
    for j in 0..(page_align_len / page_size) {
        let vaddr = page_align_start + j * page_size;
        let start_vpn: VirtPageNum = VirtAddr::from(vaddr).floor();
        let t = page_table.translate(start_vpn);
        match t {
            
            Some(tt) => {
                if tt.is_valid(){
                    print!("delete {:x}\n",vaddr);
                    page_table.remove_area_with_start_vpn(start_vpn);
                    
                }
                else{
                    print!("not delete{:x}\n",vaddr);
                    flag=-1;
                }
            }
            None => {
                print!("not delete{:x}\n",vaddr);
                flag=-1;
            }
            
        }
    }
    flag
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
   // trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",0
       // current_task().unwrap().pid.0
    );
  //  print!("spawn\n");
  //  print!("fork\n");
    let current_task1 = current_task().unwrap();
    let new_task = current_task1.fork();
    let new_pid = new_task.pid.0;
    add_task(new_task.clone());
   // print!("fork success new_pid:{}\n",new_pid);
    let mut flag= -1;
    let token = current_user_token();
    let path = translated_str(token, path);
   // let current_task = current_task().unwrap();
    if let Some(data) = get_app_data_by_name(path.as_str()) {
       // print!("exec spawn child\n");
        new_task.exec(data);
       // current_task.exec(data);
        flag=0;
    } 
    
    if flag==-1{
        return -1;
    }
   // print!("spawn end\n");
    new_pid as isize  

}

const  BIG_STRIDE:usize=10000;
// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
   // print!("setpro\n");
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",0
       // current_task().unwrap().pidz.0
    );
   // print!("setpr");
    // 检查输入是否合法
    if prio < 2 {
        return -1;
    }
    
    // 获取当前进程的 TCB
   let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    
    // 更新进程的优先级和 stride
    inner.priority = prio as usize;
    inner.stride = BIG_STRIDE / inner.priority;
    
    // 返回进程的优先级
    inner.priority as isize
}
