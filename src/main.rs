use mach2::{
    kern_return::KERN_SUCCESS,
    port::mach_port_t,
    traps::{mach_task_self, task_for_pid},
    vm::mach_vm_region,
    vm_region::{vm_region_submap_info, VM_REGION_BASIC_INFO},
    vm_types::mach_vm_address_t,
};

use clap::Parser;
use psutil::process;
use std::{
    collections::HashSet, error::Error, fmt, mem::size_of, ptr::addr_of_mut, sync::OnceLock,
};

use libc::getuid;

extern "C" {
    fn csops(pid: u32, code: u32, status: *const u32, size: u32) -> i32;
}

type AslrSlide = u32;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ProcessDetails {
    #[arg(long, default_value_t = 0)]
    pid: u32,
    #[arg(long, default_value_t = 0x100000000)]
    base_address: mach_vm_address_t,
    #[arg(long)]
    name: String,
}

const CS_RUNTIME: u32 = 0x10000;
const RED_COLOR: &str = "\x1b[0;31m";
const BLUE_COLOR: &str = "\x1b[0;34m";
const RESET_COLOR: &str = "\x1b[0m";

static BASE_ADDRESS: OnceLock<mach_vm_address_t> = OnceLock::new();

pub(crate) fn check_hardened_runtime(pid: u32) -> bool {
    let mut status: u32 = 0;

    unsafe { csops(pid, 0, addr_of_mut!(status), 4) };

    (status & CS_RUNTIME) == CS_RUNTIME
}

fn fetch_target_pids(name: &str) -> Result<HashSet<u32>, Box<dyn Error>> {
    let process_collector = process::ProcessCollector::new()?;

    let process_list = process_collector.processes;

    let mut pid_list: HashSet<u32> = HashSet::new();

    process_list
        .into_iter()
        .filter(|(_, process)| process.name().is_ok_and(|proc_name| proc_name == name))
        .for_each(|(pid, _)| {
            pid_list.insert(pid);
        });

    if !pid_list.is_empty() {
        Ok(pid_list)
    } else {
        Err(Box::new(TaskFindError))
    }
}

#[derive(Debug)]
struct TaskFindError;

impl Error for TaskFindError {}

impl fmt::Display for TaskFindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ {RED_COLOR}x{RESET_COLOR} ] Could not find task! Please check if hardened runtime is disabled for the running task.")
    }
}

#[derive(Debug)]
struct TaskGrabError;

impl Error for TaskGrabError {}

impl fmt::Display for TaskGrabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ {RED_COLOR}x{RESET_COLOR} ] Could not grab task!")
    }
}

fn task_to_slide(task: mach_port_t) -> Option<AslrSlide> {
    let mut region: vm_region_submap_info = vm_region_submap_info::default();
    let mut address: mach_vm_address_t = 0;
    let mut size: u64 = 0;
    let mut object_name: u32 = 0;
    let base_address: mach_vm_address_t = *BASE_ADDRESS.get().unwrap();

    let status = unsafe {
        mach_vm_region(
            task,
            addr_of_mut!(address),
            addr_of_mut!(size),
            VM_REGION_BASIC_INFO,
            addr_of_mut!(region) as _,
            &mut (size_of::<vm_region_submap_info>() as u32),
            addr_of_mut!(object_name),
        )
    };

    ((status == KERN_SUCCESS) && (address > base_address))
        .then(|| (address - base_address) as AslrSlide)
}

fn pid_to_task(pid: u32) -> Result<mach_port_t, TaskGrabError> {
    let mut target_task: mach_port_t = 0;

    if unsafe { task_for_pid(mach_task_self(), pid as i32, addr_of_mut!(target_task)) }
        == KERN_SUCCESS
    {
        Ok(target_task)
    } else {
        Err(TaskGrabError)
    }
}

fn main() {
    if let Ok(args) = ProcessDetails::try_parse() {
        if unsafe { getuid() } != 0 {
            println!("[ {RED_COLOR}x{RESET_COLOR} ] Please run this program as root!");
        }

        if args.base_address != 0x100000000 {
            let _ = *BASE_ADDRESS.get_or_init(|| args.base_address);
        } else {
            let _ = *BASE_ADDRESS.get_or_init(|| 0x100000000);
        }

        if args.pid != 0 {
            let pid = args.pid;

            let task = pid_to_task(pid).unwrap();

            if let Some(slide) = task_to_slide(task) {
                println!("[{BLUE_COLOR}PID{RESET_COLOR}: {pid}] ASLR Slide: {slide:#x}");
            } else {
                println!("[ {RED_COLOR}x{RESET_COLOR} ] Could not grab ASLR slide! Please check if the base address is correct.")
            }
        } else {
            let pid_list = fetch_target_pids(args.name.as_str()).unwrap();

            for pid in pid_list.iter().filter(|pid| !check_hardened_runtime(**pid)) {
                let task = pid_to_task(*pid).unwrap();

                if let Some(slide) = task_to_slide(task) {
                    println!("[{BLUE_COLOR}PID{RESET_COLOR}: {pid}] ASLR Slide: {slide:#x}");
                } else {
                    println!("[ {RED_COLOR}x{RESET_COLOR} ] Could not grab ASLR slide! Please check if the base address is correct.")
                }
            }
        }
    } else {
        println!("Usage: \n./apple_aslrtool --name=\n./apple_aslrtool --pid=\n\nOptions: --name=, --pid=, --base-address (OPTIONAL), --help");
    }
}
