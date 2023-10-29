//! Process management syscalls





use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token, TASK_MANAGER,
    }, mm::{  translated_byte_buffer, MapPermission, VirtAddr, VirtPageNum,}, timer::get_time_us, syscall::{SYSCALL_GET_TIME, SYSCALL_YIELD, SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_TASK_INFO},
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
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
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
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let page_table = &mut inner.tasks[current].memory_set;
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
            print!("insert {:x}\n",vaddr);
            page_table.insert_framed_area(
            VirtAddr::from(vaddr),
            VirtAddr::from(vaddr+page_size-1),
                permission,
            );
        }
    }
    flag
}
//static  mut TOTAL:i32=0;
// YOUR JOB: Implement mmap.
/* pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
  //  assert_eq!(0x10000000,start);
    let page_size = 4096;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);
    let start_vpn: VirtPageNum = VirtAddr::from(start).floor();
    let end_vpn: VirtPageNum = VirtAddr::from(start+len).ceil();
    print!("start:{:x},page_align_start:{:x},page_align_len:{}\n",start,page_align_start,page_align_len);
    let start_floor:usize = start_vpn.into();
    let end_floor:usize = end_vpn.into();
    print!("start_vpn:{:x},end_vpn:{:x}\n",start_floor,end_floor);
    let start_vir=VirtAddr::from(start);
    let end_vir=VirtAddr::from(end_vpn);
    let start_floor2:usize = start_vir.into();
    let end_floor2:usize = end_vir.into();
    print!("start_vir:{:x},end_vir:{:x}\n",start_floor2,end_floor2);
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
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let page_table = &mut inner.tasks[current].user_memory_set;
    let mut flag=0;
    for i in 0..(page_align_len / page_size) {
      //  unsafe{
          //  TOTAL+=1;
         //   print!("total:{}\n",TOTAL);
     //   }
        let vaddr = page_align_start + i * page_size;
        let start_vpn: VirtPageNum = VirtAddr::from(vaddr).floor();
        let end_vpn: VirtPageNum = VirtAddr::from(vaddr+page_size).ceil();
       // assert_eq!(start_vpn,end_vpn);
        let t = page_table.translate(start_vpn);
        match t {
            Some(_) => {
              //  print!("error\n");
                flag = -1;
            }
            None => {
              //  print!("{},{}\n ",start_vpn.into().into(),end_vpn.into());
                assert_ne!(start_vpn,end_vpn);
                let ttttt:VirtAddr=end_vpn.into();
                let ttttt2=VirtAddr::from(vaddr+page_size);
                if ttttt!=ttttt2 {
                    print!("error\n");
                }
                page_table.insert_framed_area(
                    VirtAddr::from(start_vpn),
                    VirtAddr::from(vaddr+page_size),
                    permission,
                );
                let tt=page_table.translate(start_vpn);
                
                match tt {
                    Some(t)=>{
                        if t.is_valid() {
                            let ttttttt:usize=PhysAddr::from(t.ppn()).into();
                            print!("{:x}",ttttttt);
                            if t.writable() {
                                
                                print!("Success");
                            }else{
                                print!("Error");
                            }
                            }
                        else {print!("Error");}
                    }
                    None=>{
                        print!("Error");
                    }
                }
            }
        }
    } 
    flag
} */

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let page_size = 4096;
    let page_align_start = start & !(page_size - 1);
    let page_align_len = (len + page_size - 1) & !(page_size - 1);

    if start != page_align_start {
        return -1;
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let page_table = &mut inner.tasks[current].memory_set;
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
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
