mod move_inst;
#[cfg(target_arch = "x86_64")]
mod tests;

use std::io::{Cursor, Seek, SeekFrom, Write};
use std::slice;

#[cfg(windows)]
use core::ffi::c_void;
#[cfg(unix)]
use libc::{__errno_location, c_void, mprotect, sysconf};
#[cfg(windows)]
use windows_sys::Win32::Foundation::GetLastError;
#[cfg(windows)]
use windows_sys::Win32::System::Memory::VirtualProtect;

use bitflags::bitflags;
use iced_x86::{Decoder, DecoderOptions, Instruction};

use crate::HookError;
use move_inst::move_code_to_addr;

const MAX_INST_LEN: usize = 15;

const TRAMPOLINE_MAX_LEN: usize = 1024;

 
 
 
 
 
 
 
pub type JmpBackRoutine = unsafe extern "win64" fn(regs: *mut Registers, user_data: usize);

 
 
 
 
 
 
 
 
 
 
 
 
 
pub type RetnRoutine =
    unsafe extern "win64" fn(regs: *mut Registers, ori_func_ptr: usize, user_data: usize) -> usize;

 
 
 
 
 
 
 
 
pub type JmpToAddrRoutine =
    unsafe extern "win64" fn(regs: *mut Registers, ori_func_ptr: usize, user_data: usize);

 
 
 
 
 
 
 
 
 
 
 
 
pub type JmpToRetRoutine =
    unsafe extern "win64" fn(regs: *mut Registers, ori_func_ptr: usize, user_data: usize) -> usize;

 
pub enum HookType {
     
    JmpBack(JmpBackRoutine),

     
    Retn(RetnRoutine),

     
    JmpToAddr(usize, JmpToAddrRoutine),

     
    JmpToRet(JmpToRetRoutine),
}

 
pub enum JmpType {
     
     
    Direct,

     
     
    MovJmp,

     
     
    TrampolineJmp(usize),
}

 
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Registers {
     
    pub xmm0: u128,
     
    pub xmm1: u128,
     
    pub xmm2: u128,
     
    pub xmm3: u128,
     
    pub r15: u64,
     
    pub r14: u64,
     
    pub r13: u64,
     
    pub r12: u64,
     
    pub r11: u64,
     
    pub r10: u64,
     
    pub r9: u64,
     
    pub r8: u64,
     
    pub rbp: u64,
     
    pub rdi: u64,
     
    pub rsi: u64,
     
    pub rdx: u64,
     
    pub rcx: u64,
     
    pub rbx: u64,
     
    pub rsp: u64,
     
    pub rflags: u64,
     
    pub _no_use: u64,
     
    pub rax: u64,
}

impl Registers {
     
     
     
     
     
     
     
     
     
    #[must_use]
    pub unsafe fn get_stack(&self, cnt: usize) -> u64 {
        unsafe { *((self.rsp as usize + cnt * 8) as *mut u64) }
    }
}

 
 
pub trait ThreadCallback {
     
    fn pre(&self) -> bool;
     
    fn post(&self);
}

 
pub enum CallbackOption {
     
    Some(Box<dyn ThreadCallback>),
     
    None,
}

bitflags! {
     
    pub struct HookFlags:u32 {
         
         
        const NOT_MODIFY_MEMORY_PROTECT = 0x1;
    }
}

 
 
pub struct Hooker {
    addr: usize,
    hook_type: HookType,
    thread_cb: CallbackOption,
    user_data: usize,
    jmp_inst_size: usize,
    flags: HookFlags,
}

 
pub struct HookPoint {
    addr: usize,
    #[allow(dead_code)]  
    trampoline: Box<[u8; TRAMPOLINE_MAX_LEN]>,
    trampoline_prot: u32,
    origin: Vec<u8>,
    thread_cb: CallbackOption,
    jmp_inst_size: usize,
    flags: HookFlags,
}

 
 
 
pub struct ClosureHookPoint<'a> {
    _inner: HookPoint,
    _callback: Box<dyn HookCallback + 'a>,
}

 
 
trait HookCallback {}
impl<T> HookCallback for T {}

#[cfg(not(target_arch = "x86_64"))]
fn env_lock() {
    panic!("This crate should only be used in arch x86_32!")
}
#[cfg(target_arch = "x86_64")]
fn env_lock() {}

impl Hooker {
     
     
     
     
     
     
     
     
    #[must_use]
    pub fn new(
        addr: usize,
        hook_type: HookType,
        thread_cb: CallbackOption,
        user_data: usize,
        flags: HookFlags,
    ) -> Self {
        env_lock();
        Self {
            addr,
            hook_type,
            thread_cb,
            user_data,
            jmp_inst_size: 14,
            flags,
        }
    }

     
     
     
     
     
     
     
     
     
     
     
    pub unsafe fn hook(self) -> Result<HookPoint, HookError> {
        let (moving_insts, origin) = get_moving_insts(self.addr, self.jmp_inst_size)?;
        let trampoline =
            generate_trampoline(&self, moving_insts, origin.len() as u8, self.user_data)?;
        let trampoline_prot = modify_mem_protect(trampoline.as_ptr() as usize, trampoline.len())?;
        if !self.flags.contains(HookFlags::NOT_MODIFY_MEMORY_PROTECT) {
            let old_prot = modify_mem_protect(self.addr, self.jmp_inst_size)?;
            let ret = modify_jmp_with_thread_cb(&self, trampoline.as_ptr() as usize);
            recover_mem_protect(self.addr, self.jmp_inst_size, old_prot);
            ret?;
        } else {
            modify_jmp_with_thread_cb(&self, trampoline.as_ptr() as usize)?;
        }
        Ok(HookPoint {
            addr: self.addr,
            trampoline,
            trampoline_prot,
            origin,
            thread_cb: self.thread_cb,
            jmp_inst_size: self.jmp_inst_size,
            flags: self.flags,
        })
    }
}


 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_back<'a, T: Fn(*mut Registers) + Send + Sync + 'a>(
    address: usize,
    callback: T,
    callback_option: CallbackOption,
    hook_flags: HookFlags,
) -> Result<ClosureHookPoint<'a>, HookError> {
    let callback = Box::new(callback);
    let hooker = Hooker::new(
        address,
        HookType::JmpBack(run_jmp_back_closure::<T>),
        callback_option,
        &*callback as *const T as usize,
        hook_flags,
    );
    Ok(ClosureHookPoint {
        _inner: unsafe { hooker.hook()? },
        _callback: callback,
    })
}

 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_retn<
    'a,
    T: (Fn(*mut Registers, usize) -> usize) + Send + Sync + 'a,
>(
    address: usize,
    callback: T,
    callback_option: CallbackOption,
    hook_flags: HookFlags,
) -> Result<ClosureHookPoint<'a>, HookError> {
    let callback = Box::new(callback);
    let hooker = Hooker::new(
        address,
        HookType::Retn(run_retn_closure::<T>),
        callback_option,
        &*callback as *const T as usize,
        hook_flags,
    );
    Ok(ClosureHookPoint {
        _inner: unsafe { hooker.hook()? },
        _callback: callback,
    })
}

 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_to_addr<'a, T: Fn(*mut Registers, usize) + Send + Sync + 'a>(
    address: usize,
    follow_up: usize,
    callback: T,
    callback_option: CallbackOption,
    hook_flags: HookFlags,
) -> Result<ClosureHookPoint<'a>, HookError> {
    let callback = Box::new(callback);
    let hooker = Hooker::new(
        address,
        HookType::JmpToAddr(follow_up, run_jmp_to_addr_closure::<T>),
        callback_option,
        &*callback as *const T as usize,
        hook_flags,
    );
    Ok(ClosureHookPoint {
        _inner: unsafe { hooker.hook()? },
        _callback: callback,
    })
}

 
 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_to_ret<
    'a,
    T: (Fn(*mut Registers, usize) -> usize) + Send + Sync + 'a,
>(
    address: usize,
    callback: T,
    callback_option: CallbackOption,
    hook_flags: HookFlags,
) -> Result<ClosureHookPoint<'a>, HookError> {
    let callback = Box::new(callback);
    let hooker = Hooker::new(
        address,
        HookType::JmpToRet(run_retn_closure::<T>),
        callback_option,
        &*callback as *const T as usize,
        hook_flags,
    );
    Ok(ClosureHookPoint {
        _inner: unsafe { hooker.hook()? },
        _callback: callback,
    })
}

 
unsafe extern "win64" fn run_jmp_back_closure<T: Fn(*mut Registers)>(
    reg: *mut Registers,
    callback: usize,
) {
    unsafe { (*(callback as *const T))(reg) };
}

 
 
unsafe extern "win64" fn run_retn_closure<T: Fn(*mut Registers, usize) -> usize>(
    reg: *mut Registers,
    original: usize,
    callback: usize,
) -> usize {
    unsafe { (*(callback as *const T))(reg, original) }
}

 
unsafe extern "win64" fn run_jmp_to_addr_closure<T: Fn(*mut Registers, usize)>(
    reg: *mut Registers,
    original: usize,
    callback: usize,
) {
    unsafe { (*(callback as *const T))(reg, original) };
}

impl HookPoint {
     
    pub unsafe fn unhook(self) -> Result<(), HookError> {
        self.unhook_by_ref()
    }

    fn unhook_by_ref(&self) -> Result<(), HookError> {
        let ret: Result<(), HookError>;
        if !self.flags.contains(HookFlags::NOT_MODIFY_MEMORY_PROTECT) {
            let old_prot = modify_mem_protect(self.addr, self.jmp_inst_size)?;
            ret = recover_jmp_with_thread_cb(self);
            recover_mem_protect(self.addr, self.jmp_inst_size, old_prot);
        } else {
            ret = recover_jmp_with_thread_cb(self)
        }
        recover_mem_protect(
            self.trampoline.as_ptr() as usize,
            self.trampoline.len(),
            self.trampoline_prot,
        );
        ret
    }
}

 
impl Drop for HookPoint {
    fn drop(&mut self) {
        self.unhook_by_ref().unwrap_or_default();
    }
}

fn get_moving_insts(
    addr: usize,
    min_bytes: usize,
) -> Result<(Vec<Instruction>, Vec<u8>), HookError> {
    let code_slice = unsafe { slice::from_raw_parts(addr as *const u8, MAX_INST_LEN * 2) };
    let mut decoder = Decoder::new(64, code_slice, DecoderOptions::NONE);
    decoder.set_ip(addr as u64);

    let mut total_bytes = 0;
    let mut ori_insts: Vec<Instruction> = vec![];
    for inst in &mut decoder {
        if inst.is_invalid() {
            return Err(HookError::Disassemble);
        }
        ori_insts.push(inst);
        total_bytes += inst.len();
        if total_bytes >= min_bytes {
            break;
        }
    }

    Ok((ori_insts, code_slice[0..decoder.position()].into()))
}

fn write_trampoline_prolog(buf: &mut impl Write) -> Result<usize, std::io::Error> {
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
    buf.write(&[
        0x54, 0x9C, 0x48, 0xF7, 0xC4, 0x08, 0x00, 0x00, 0x00, 0x74, 0x2C, 0x50, 0x48, 0x83, 0xEC,
        0x10, 0x48, 0x8B, 0x44, 0x24, 0x20, 0x48, 0x89, 0x04, 0x24, 0x48, 0x8B, 0x44, 0x24, 0x18,
        0x48, 0x89, 0x44, 0x24, 0x08, 0x48, 0x8B, 0x44, 0x24, 0x10, 0x48, 0x89, 0x44, 0x24, 0x18,
        0xC7, 0x44, 0x24, 0x10, 0x01, 0x00, 0x00, 0x00, 0xEB, 0x27, 0x50, 0x50, 0x48, 0x8B, 0x44,
        0x24, 0x18, 0x48, 0x89, 0x04, 0x24, 0x48, 0x8B, 0x44, 0x24, 0x08, 0x48, 0x89, 0x44, 0x24,
        0x18, 0x48, 0x8B, 0x44, 0x24, 0x10, 0x48, 0x89, 0x44, 0x24, 0x08, 0xC7, 0x44, 0x24, 0x10,
        0x00, 0x00, 0x00, 0x00, 0x53, 0x51, 0x52, 0x56, 0x57, 0x55, 0x41, 0x50, 0x41, 0x51, 0x41,
        0x52, 0x41, 0x53, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x40,
        0x0F, 0x29, 0x04, 0x24, 0x0F, 0x29, 0x4C, 0x24, 0x10, 0x0F, 0x29, 0x54, 0x24, 0x20, 0x0F,
        0x29, 0x5C, 0x24, 0x30,
    ])
}

fn write_trampoline_epilog1(buf: &mut impl Write) -> Result<usize, std::io::Error> {
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
    buf.write(&[
        0x0F, 0x28, 0x04, 0x24, 0x0F, 0x28, 0x4C, 0x24, 0x10, 0x0F, 0x28, 0x54, 0x24, 0x20, 0x0F,
        0x28, 0x5C, 0x24, 0x30, 0x48, 0x83, 0xC4, 0x40, 0x41, 0x5F, 0x41, 0x5E, 0x41, 0x5D, 0x41,
        0x5C, 0x41, 0x5B, 0x41, 0x5A, 0x41, 0x59, 0x41, 0x58, 0x5D, 0x5F, 0x5E, 0x5A, 0x59, 0x5B,
        0x48, 0x83, 0xC4, 0x08,
    ])
}

fn write_trampoline_epilog2_common(buf: &mut impl Write) -> Result<usize, std::io::Error> {
     
     
     
     
     
     
     
     
     
     
     
     
     
     
    buf.write(&[
        0xF7, 0x44, 0x24, 0x08, 0x01, 0x00, 0x00, 0x00, 0x74, 0x10, 0x48, 0x8B, 0x44, 0x24, 0x10,
        0x48, 0x89, 0x44, 0x24, 0x18, 0x9D, 0x58, 0x58, 0x58, 0xEB, 0x03, 0x9D, 0x58, 0x58,
    ])
}

fn write_trampoline_epilog2_jmp_ret(buf: &mut impl Write) -> Result<usize, std::io::Error> {
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
    buf.write(&[
        0xF7, 0x44, 0x24, 0x08, 0x01, 0x00, 0x00, 0x00, 0x74, 0x0E, 0x9D, 0x48, 0x89, 0x04, 0x24,
        0x58, 0x58, 0x48, 0x8D, 0x64, 0x24, 0x08, 0xEB, 0x08, 0x9D, 0x48, 0x89, 0x44, 0x24, 0xF8,
        0x58, 0x58, 0xFF, 0x64, 0x24, 0xE8
    ])
}

fn jmp_addr<T: Write>(addr: u64, buf: &mut T) -> Result<(), HookError> {
    buf.write(&[0xff, 0x25, 0, 0, 0, 0])?;
    buf.write(&addr.to_le_bytes())?;
    Ok(())
}

fn write_ori_func_addr<T: Write + Seek>(buf: &mut T, ori_func_addr_off: u64, ori_func_off: u64) {
    let pos = buf.stream_position().unwrap();
    buf.seek(SeekFrom::Start(ori_func_addr_off)).unwrap();
    buf.write(&ori_func_off.to_le_bytes()).unwrap();
    buf.seek(SeekFrom::Start(pos)).unwrap();
}

fn generate_jmp_back_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u64,
    moving_code: &Vec<Instruction>,
    ori_addr: usize,
    cb: JmpBackRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x48, 0xba])?;
    buf.write(&(user_data as u64).to_le_bytes())?;
     
     
     
    buf.write(&[0x48, 0x89, 0xe1, 0x48, 0x83, 0xec, 0x10, 0x48, 0xb8])?;
    buf.write(&(cb as usize as u64).to_le_bytes())?;
     
     
    buf.write(&[0xff, 0xd0, 0x48, 0x83, 0xc4, 0x10])?;
    write_trampoline_epilog1(buf)?;
    write_trampoline_epilog2_common(buf)?;

    let cur_pos = buf.stream_position().unwrap();
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + cur_pos,
    )?)?;

    jmp_addr(ori_addr as u64 + u64::from(ori_len), buf)?;
    Ok(())
}

fn generate_retn_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u64,
    moving_code: &Vec<Instruction>,
    ori_addr: usize,
    cb: RetnRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x49, 0xb8])?;
    buf.write(&(user_data as u64).to_le_bytes())?;
    let ori_func_addr_off = buf.stream_position().unwrap() + 2;
     
     
     
     
    buf.write(&[
        0x48, 0xba, 0, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x89, 0xe1, 0x48, 0x83, 0xec, 0x20, 0x48, 0xb8,
    ])?;
    buf.write(&(cb as usize as u64).to_le_bytes())?;
     
     
     
    buf.write(&[
        0xff, 0xd0, 0x48, 0x83, 0xc4, 0x20, 0x48, 0x89, 0x84, 0x24, 0xc8, 0x00, 0x00, 0x00,
    ])?;
    write_trampoline_epilog1(buf)?;
    write_trampoline_epilog2_common(buf)?;
     
    buf.write(&[0xc3])?;

    let ori_func_off = buf.stream_position().unwrap();
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + ori_func_off,
    )?)?;
    jmp_addr(ori_addr as u64 + u64::from(ori_len), buf)?;

    write_ori_func_addr(buf, ori_func_addr_off, trampoline_base_addr + ori_func_off);

    Ok(())
}

fn generate_jmp_addr_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u64,
    moving_code: &Vec<Instruction>,
    ori_addr: usize,
    dest_addr: usize,
    cb: JmpToAddrRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x49, 0xb8])?;
    buf.write(&(user_data as u64).to_le_bytes())?;
    let ori_func_addr_off = buf.stream_position().unwrap() + 2;
     
     
     
     
    buf.write(&[
        0x48, 0xba, 0, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x89, 0xe1, 0x48, 0x83, 0xec, 0x20, 0x48, 0xb8,
    ])?;
    buf.write(&(cb as usize as u64).to_le_bytes())?;
     
     
    buf.write(&[0xff, 0xd0, 0x48, 0x83, 0xc4, 0x20])?;
    write_trampoline_epilog1(buf)?;
    write_trampoline_epilog2_common(buf)?;
    jmp_addr(dest_addr as u64, buf)?;

    let ori_func_off = buf.stream_position().unwrap();
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + ori_func_off,
    )?)?;
    jmp_addr(ori_addr as u64 + u64::from(ori_len), buf)?;

    write_ori_func_addr(buf, ori_func_addr_off, trampoline_base_addr + ori_func_off);

    Ok(())
}

fn generate_jmp_ret_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u64,
    moving_code: &Vec<Instruction>,
    ori_addr: usize,
    cb: JmpToRetRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x49, 0xb8])?;
    buf.write(&(user_data as u64).to_le_bytes())?;
    let ori_func_addr_off = buf.stream_position().unwrap() + 2;
     
     
     
     
    buf.write(&[
        0x48, 0xba, 0, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x89, 0xe1, 0x48, 0x83, 0xec, 0x20, 0x48, 0xb8,
    ])?;
    buf.write(&(cb as usize as u64).to_le_bytes())?;
     
     
    buf.write(&[0xff, 0xd0, 0x48, 0x83, 0xc4, 0x20])?;
    write_trampoline_epilog1(buf)?;
    write_trampoline_epilog2_jmp_ret(buf)?;

    let ori_func_off = buf.stream_position().unwrap();
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + ori_func_off,
    )?)?;
    jmp_addr(ori_addr as u64 + u64::from(ori_len), buf)?;

    write_ori_func_addr(buf, ori_func_addr_off, trampoline_base_addr + ori_func_off);

    Ok(())
}

fn generate_trampoline(
    hooker: &Hooker,
    moving_code: Vec<Instruction>,
    ori_len: u8,
    user_data: usize,
) -> Result<Box<[u8; TRAMPOLINE_MAX_LEN]>, HookError> {
    let mut trampoline_buffer = Box::new([0u8; TRAMPOLINE_MAX_LEN]);
    let trampoline_addr = trampoline_buffer.as_ptr() as u64;
    let mut buf = Cursor::new(&mut trampoline_buffer[..]);

    write_trampoline_prolog(&mut buf)?;

    match hooker.hook_type {
        HookType::JmpBack(cb) => generate_jmp_back_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr,
            cb,
            ori_len,
            user_data,
        ),
        HookType::Retn(cb) => generate_retn_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr,
            cb,
            ori_len,
            user_data,
        ),
        HookType::JmpToAddr(dest_addr, cb) => generate_jmp_addr_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr,
            dest_addr,
            cb,
            ori_len,
            user_data,
        ),
        HookType::JmpToRet(cb) => generate_jmp_ret_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr,
            cb,
            ori_len,
            user_data,
        ),
    }?;

    Ok(trampoline_buffer)
}

#[cfg(windows)]
fn modify_mem_protect(addr: usize, len: usize) -> Result<u32, HookError> {
    let mut old_prot: u32 = 0;
    let old_prot_ptr = std::ptr::addr_of_mut!(old_prot);
     
    let ret = unsafe { VirtualProtect(addr as *const c_void, len, 0x40, old_prot_ptr) };
    if ret == 0 {
        Err(HookError::MemoryProtect(unsafe { GetLastError() }))
    } else {
        Ok(old_prot)
    }
}

#[cfg(unix)]
fn modify_mem_protect(addr: usize, len: usize) -> Result<u32, HookError> {
    if len == 0 {
        return Err(HookError::InvalidParameter);
    }
    let page_size = unsafe { sysconf(30) } as usize;  
     
     
     
     
    let start = addr & !(page_size - 1);
    let end = (addr + len + page_size - 1) & !(page_size - 1);
     
    let ret = unsafe { mprotect(start as *mut c_void, end - start, 7) };
    if ret != 0 {
        let err = unsafe { *(__errno_location()) };
        Err(HookError::MemoryProtect(err as u32))
    } else {
         
        Ok(7)
    }
}
#[cfg(windows)]
fn recover_mem_protect(addr: usize, len: usize, old: u32) {
    let mut old_prot: u32 = 0;
    let old_prot_ptr = std::ptr::addr_of_mut!(old_prot);
    unsafe { VirtualProtect(addr as *const c_void, len, old, old_prot_ptr) };
}

#[cfg(unix)]
fn recover_mem_protect(addr: usize, len: usize, old: u32) {
    let page_size = unsafe { sysconf(30) } as usize;  
    let start = addr & !(page_size - 1);
    let end = (addr + len.max(1) + page_size - 1) & !(page_size - 1);
    unsafe { mprotect(start as *mut c_void, end - start, old as i32) };
}
fn modify_jmp(dest_addr: usize, trampoline_addr: usize) {
    let buf = unsafe { slice::from_raw_parts_mut(dest_addr as *mut u8, 14) };
    let distance = trampoline_addr as i64 - (dest_addr as i64 + 5);
    if distance.abs() <= 0x7fff_ffff {
         
        buf[0] = 0xe9;
        buf[1..5].copy_from_slice(&(distance as i32).to_le_bytes());
    } else {
         
        buf[0..6].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
        buf[6..14].copy_from_slice(&(trampoline_addr as u64).to_le_bytes());
    }
}

fn modify_jmp_with_thread_cb(hook: &Hooker, trampoline_addr: usize) -> Result<(), HookError> {
    if let CallbackOption::Some(cbs) = &hook.thread_cb {
        if !cbs.pre() {
            return Err(HookError::PreHook);
        }
        modify_jmp(hook.addr, trampoline_addr);
        cbs.post();
        Ok(())
    } else {
        modify_jmp(hook.addr, trampoline_addr);
        Ok(())
    }
}

fn recover_jmp(dest_addr: usize, origin: &[u8]) {
    let buf = unsafe { slice::from_raw_parts_mut(dest_addr as *mut u8, origin.len()) };
     
    buf.copy_from_slice(origin);
}

fn recover_jmp_with_thread_cb(hook: &HookPoint) -> Result<(), HookError> {
    if let CallbackOption::Some(cbs) = &hook.thread_cb {
        if !cbs.pre() {
            return Err(HookError::PreHook);
        }
        recover_jmp(hook.addr, &hook.origin);
        cbs.post();
    } else {
        recover_jmp(hook.addr, &hook.origin);
    }
    Ok(())
}
