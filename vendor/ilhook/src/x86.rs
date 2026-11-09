use bitflags::bitflags;
use iced_x86::{
    BlockEncoder, BlockEncoderOptions, Decoder, DecoderOptions, Instruction, InstructionBlock,
};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::slice;

#[cfg(windows)]
use core::ffi::c_void;
#[cfg(windows)]
use windows_sys::Win32::Foundation::GetLastError;
#[cfg(windows)]
use windows_sys::Win32::System::Memory::VirtualProtect;

#[cfg(unix)]
use libc::{__errno_location, c_void, mprotect, sysconf};

use crate::err::HookError;

const MAX_INST_LEN: usize = 15;
const JMP_INST_SIZE: usize = 5;

 
 
 
 
 
 
 
pub type JmpBackRoutine = unsafe extern "cdecl" fn(regs: *mut Registers, user_data: usize);

 
 
 
 
 
 
 
 
 
 
 
 
 
pub type RetnRoutine =
    unsafe extern "cdecl" fn(regs: *mut Registers, ori_func_ptr: usize, user_data: usize) -> usize;

 
 
 
 
 
 
 
 
pub type JmpToAddrRoutine =
    unsafe extern "cdecl" fn(regs: *mut Registers, ori_func_ptr: usize, src_addr: usize);

 
 
 
 
 
 
 
 
 
 
 
 
pub type JmpToRetRoutine =
    unsafe extern "cdecl" fn(regs: *mut Registers, ori_func_ptr: usize, src_addr: usize) -> usize;

 
pub enum HookType {
     
    JmpBack(JmpBackRoutine),

     
     
    Retn(usize, RetnRoutine),

     
    JmpToAddr(usize, JmpToAddrRoutine),

     
    JmpToRet(JmpToRetRoutine),
}

 
#[repr(C)]
#[derive(Debug)]
pub struct Registers {
     
    pub eflags: u32,
     
    pub edi: u32,
     
    pub esi: u32,
     
    pub ebp: u32,
     
    pub esp: u32,
     
    pub ebx: u32,
     
    pub edx: u32,
     
    pub ecx: u32,
     
    pub eax: u32,
}

impl Registers {
     
     
     
     
     
     
     
     
     
    #[must_use]
    pub unsafe fn get_arg(&self, cnt: usize) -> u32 {
        unsafe { *((self.esp as usize + cnt * 4) as *mut u32) }
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
    flags: HookFlags,
    user_data: usize,
}

 
pub struct HookPoint {
    addr: usize,
    trampoline: Box<[u8; 100]>,
    trampoline_prot: u32,
    origin: Vec<u8>,
    thread_cb: CallbackOption,
    flags: HookFlags,
}

 
 
 
pub struct ClosureHookPoint<'a> {
    _inner: HookPoint,
    _callback: Box<dyn HookCallback + 'a>,
}

 
 
trait HookCallback {}
impl<T> HookCallback for T {}

#[cfg(not(target_arch = "x86"))]
fn env_lock() {
    panic!("This crate should only be used in arch x86_32!")
}
#[cfg(target_arch = "x86")]
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
            flags,
        }
    }

     
     
     
     
     
     
     
     
     
     
     
     
    pub unsafe fn hook(self) -> Result<HookPoint, HookError> {
        let (moving_insts, origin) = get_moving_insts(self.addr)?;
        let trampoline =
            generate_trampoline(&self, moving_insts, origin.len() as u8, self.user_data)?;
        let trampoline_prot = modify_mem_protect(trampoline.as_ptr() as usize, trampoline.len())?;
        if !self.flags.contains(HookFlags::NOT_MODIFY_MEMORY_PROTECT) {
            let old_prot = modify_mem_protect(self.addr, JMP_INST_SIZE)?;
            let ret = modify_jmp_with_thread_cb(&self, trampoline.as_ptr() as usize);
            recover_mem_protect(self.addr, JMP_INST_SIZE, old_prot);
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
            flags: self.flags,
        })
    }
}

 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_back<'a, T: Fn(*mut Registers) + 'a>(
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

 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_retn<'a, T: (Fn(*mut Registers, usize) -> usize) + 'a>(
    address: usize,
    mnemonic: usize,
    callback: T,
    callback_option: CallbackOption,
    hook_flags: HookFlags,
) -> Result<ClosureHookPoint<'a>, HookError> {
    let callback = Box::new(callback);
    let hooker = Hooker::new(
        address,
        HookType::Retn(mnemonic, run_retn_closure::<T>),
        callback_option,
        &*callback as *const T as usize,
        hook_flags,
    );
    Ok(ClosureHookPoint {
        _inner: unsafe { hooker.hook()? },
        _callback: callback,
    })
}

 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_to_addr<'a, T: Fn(*mut Registers, usize) + 'a>(
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

 
 
 
 
 
 
 
 
 
 
 
 
 
pub unsafe fn hook_closure_jmp_to_ret<'a, T: (Fn(*mut Registers, usize) -> usize) + 'a>(
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

 
unsafe extern "cdecl" fn run_jmp_back_closure<T: Fn(*mut Registers)>(
    reg: *mut Registers,
    callback: usize,
) {
    unsafe { (*(callback as *const T))(reg) };
}

 
 
unsafe extern "cdecl" fn run_retn_closure<T: Fn(*mut Registers, usize) -> usize>(
    reg: *mut Registers,
    original: usize,
    callback: usize,
) -> usize {
    unsafe { (*(callback as *const T))(reg, original) }
}

 
unsafe extern "cdecl" fn run_jmp_to_addr_closure<T: Fn(*mut Registers, usize)>(
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
            let old_prot = modify_mem_protect(self.addr, JMP_INST_SIZE)?;
            ret = recover_jmp_with_thread_cb(self);
            recover_mem_protect(self.addr, JMP_INST_SIZE, old_prot);
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

fn get_moving_insts(addr: usize) -> Result<(Vec<Instruction>, Vec<u8>), HookError> {
    let code_slice =
        unsafe { slice::from_raw_parts(addr as *const u8, MAX_INST_LEN * JMP_INST_SIZE) };
    let mut decoder = Decoder::new(32, code_slice, DecoderOptions::NONE);
    decoder.set_ip(addr as u64);

    let mut total_bytes = 0;
    let mut ori_insts: Vec<Instruction> = vec![];
    for inst in &mut decoder {
        if inst.is_invalid() {
            return Err(HookError::Disassemble);
        }
        ori_insts.push(inst);
        total_bytes += inst.len();
        if total_bytes >= JMP_INST_SIZE {
            break;
        }
    }

    Ok((ori_insts, code_slice[0..decoder.position()].into()))
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

fn write_relative_off<T: Write + Seek>(
    buf: &mut T,
    base_addr: u32,
    dst_addr: u32,
) -> Result<(), HookError> {
    let dst_addr = dst_addr as i32;
    let cur_pos = buf.stream_position().unwrap() as i32;
    let call_off = dst_addr - (base_addr as i32 + cur_pos + 4);
    buf.write(&call_off.to_le_bytes())?;
    Ok(())
}

fn move_code_to_addr(ori_insts: &Vec<Instruction>, dest_addr: u32) -> Result<Vec<u8>, HookError> {
    let block = InstructionBlock::new(ori_insts, u64::from(dest_addr));
    let encoded = BlockEncoder::encode(32, block, BlockEncoderOptions::NONE)
        .map_err(|_| HookError::MoveCode)?;
    Ok(encoded.code_buffer)
}

fn write_ori_func_addr<T: Write + Seek>(buf: &mut T, ori_func_addr_off: u32, ori_func_off: u32) {
    let pos = buf.stream_position().unwrap();
    buf.seek(SeekFrom::Start(u64::from(ori_func_addr_off)))
        .unwrap();
    buf.write(&ori_func_off.to_le_bytes()).unwrap();
    buf.seek(SeekFrom::Start(pos)).unwrap();
}

fn generate_jmp_back_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u32,
    moving_code: &Vec<Instruction>,
    ori_addr: u32,
    cb: JmpBackRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x68])?;
    buf.write(&user_data.to_le_bytes())?;

     
     
    buf.write(&[0x55, 0xe8])?;
    write_relative_off(buf, trampoline_base_addr, cb as u32)?;

     
    buf.write(&[0x83, 0xc4, 0x08])?;
     
     
    buf.write(&[0x9d, 0x61])?;

    let cur_pos = buf.stream_position().unwrap() as u32;
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + cur_pos,
    )?)?;
     
    buf.write(&[0xe9])?;
    write_relative_off(buf, trampoline_base_addr, ori_addr + u32::from(ori_len))
}

fn generate_retn_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u32,
    moving_code: &Vec<Instruction>,
    ori_addr: u32,
    retn_val: u16,
    cb: RetnRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x68])?;
    buf.write(&user_data.to_le_bytes())?;

     
     
     
    let ori_func_addr_off = buf.stream_position().unwrap() + 1;
    buf.write(&[0x68, 0, 0, 0, 0, 0x55, 0xe8])?;
    write_relative_off(buf, trampoline_base_addr, cb as u32)?;

     
    buf.write(&[0x83, 0xc4, 0x0c])?;
     
    buf.write(&[0x89, 0x44, 0x24, 0x20])?;
     
     
    buf.write(&[0x9d, 0x61])?;
    if retn_val == 0 {
         
        buf.write(&[0xc3])?;
    } else {
         
        buf.write(&[0xc2])?;
        buf.write(&retn_val.to_le_bytes())?;
    }
    let ori_func_off = buf.stream_position().unwrap() as u32;
    write_ori_func_addr(
        buf,
        ori_func_addr_off as u32,
        trampoline_base_addr + ori_func_off,
    );

    let cur_pos = buf.stream_position().unwrap() as u32;
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + cur_pos,
    )?)?;

     
    buf.write(&[0xe9])?;
    write_relative_off(buf, trampoline_base_addr, ori_addr + u32::from(ori_len))
}

fn generate_jmp_addr_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u32,
    moving_code: &Vec<Instruction>,
    ori_addr: u32,
    dest_addr: u32,
    cb: JmpToAddrRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x68])?;
    buf.write(&user_data.to_le_bytes())?;

     
     
     
    let ori_func_addr_off = buf.stream_position().unwrap() + 1;
    buf.write(&[0x68, 0, 0, 0, 0, 0x55, 0xe8])?;
    write_relative_off(buf, trampoline_base_addr, cb as u32)?;

     
    buf.write(&[0x83, 0xc4, 0x0c])?;
     
     
    buf.write(&[0x9d, 0x61])?;
     
    buf.write(&[0xe9])?;
    write_relative_off(buf, trampoline_base_addr, dest_addr + u32::from(ori_len))?;

    let ori_func_off = buf.stream_position().unwrap() as u32;
    write_ori_func_addr(
        buf,
        ori_func_addr_off as u32,
        trampoline_base_addr + ori_func_off,
    );

    let cur_pos = buf.stream_position().unwrap() as u32;
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + cur_pos,
    )?)?;

     
    buf.write(&[0xe9])?;
    write_relative_off(buf, trampoline_base_addr, ori_addr + u32::from(ori_len))
}

fn generate_jmp_ret_trampoline<T: Write + Seek>(
    buf: &mut T,
    trampoline_base_addr: u32,
    moving_code: &Vec<Instruction>,
    ori_addr: u32,
    cb: JmpToRetRoutine,
    ori_len: u8,
    user_data: usize,
) -> Result<(), HookError> {
     
    buf.write(&[0x68])?;
    buf.write(&user_data.to_le_bytes())?;

     
     
     
    let ori_func_addr_off = buf.stream_position().unwrap() + 1;
    buf.write(&[0x68, 0, 0, 0, 0, 0x55, 0xe8])?;
    write_relative_off(buf, trampoline_base_addr, cb as u32)?;

     
    buf.write(&[0x83, 0xc4, 0x0c])?;
     
    buf.write(&[0x89, 0x44, 0x24, 0xfc])?;
     
     
    buf.write(&[0x9d, 0x61])?;
     
    buf.write(&[0xff, 0x64, 0x24, 0xd8])?;

    let ori_func_off = buf.stream_position().unwrap() as u32;
    write_ori_func_addr(
        buf,
        ori_func_addr_off as u32,
        trampoline_base_addr + ori_func_off,
    );

    let cur_pos = buf.stream_position().unwrap() as u32;
    buf.write(&move_code_to_addr(
        moving_code,
        trampoline_base_addr + cur_pos,
    )?)?;

     
    buf.write(&[0xe9])?;
    write_relative_off(buf, trampoline_base_addr, ori_addr + u32::from(ori_len))
}

fn generate_trampoline(
    hooker: &Hooker,
    moving_code: Vec<Instruction>,
    ori_len: u8,
    user_data: usize,
) -> Result<Box<[u8; 100]>, HookError> {
    let mut raw_buffer = Box::new([0u8; 100]);
    let trampoline_addr = raw_buffer.as_ptr() as u32;
    let mut buf = Cursor::new(&mut raw_buffer[..]);

     
     
     
    buf.write(&[0x60, 0x9c, 0x8b, 0xec])?;

    match hooker.hook_type {
        HookType::JmpBack(cb) => generate_jmp_back_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr as u32,
            cb,
            ori_len,
            user_data,
        ),
        HookType::Retn(val, cb) => generate_retn_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr as u32,
            val as u16,
            cb,
            ori_len,
            user_data,
        ),
        HookType::JmpToAddr(dest, cb) => generate_jmp_addr_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr as u32,
            dest as u32,
            cb,
            ori_len,
            user_data,
        ),
        HookType::JmpToRet(cb) => generate_jmp_ret_trampoline(
            &mut buf,
            trampoline_addr,
            &moving_code,
            hooker.addr as u32,
            cb,
            ori_len,
            user_data,
        ),
    }?;

    Ok(raw_buffer)
}

fn modify_jmp(dest_addr: usize, trampoline_addr: usize) -> Result<(), HookError> {
    let buf = unsafe { slice::from_raw_parts_mut(dest_addr as *mut u8, JMP_INST_SIZE) };
     
    buf[0] = 0xe9;
    let rel_off = trampoline_addr as i32 - (dest_addr as i32 + 5);
    buf[1..5].copy_from_slice(&rel_off.to_le_bytes());
    Ok(())
}

fn modify_jmp_with_thread_cb(hook: &Hooker, trampoline_addr: usize) -> Result<(), HookError> {
    if let CallbackOption::Some(cbs) = &hook.thread_cb {
        if !cbs.pre() {
            return Err(HookError::PreHook);
        }
        let ret = modify_jmp(hook.addr, trampoline_addr);
        cbs.post();
        ret
    } else {
        modify_jmp(hook.addr, trampoline_addr)
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

#[cfg(target_arch = "x86")]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(test)]
    #[inline(never)]
    fn foo(x: u32) -> u32 {
        println!("original foo, x:{}", x);
        x * x
    }
    #[cfg(test)]
    unsafe extern "cdecl" fn on_foo(
        reg: *mut Registers,
        old_func: usize,
        user_data: usize,
    ) -> usize {
        let old_func = std::mem::transmute::<usize, fn(u32) -> u32>(old_func);
        old_func((*reg).get_arg(1)) as usize + user_data
    }

    #[test]
    fn test_hook_function_cdecl() {
        assert_eq!(foo(5), 25);
        let hooker = Hooker::new(
            foo as usize,
            HookType::Retn(0, on_foo),
            CallbackOption::None,
            100,
            HookFlags::empty(),
        );
        let info = unsafe { hooker.hook().unwrap() };
        assert_eq!(foo(5), 125);
        unsafe { info.unhook().unwrap() };
        assert_eq!(foo(5), 25);
    }

    #[cfg(test)]
    #[inline(never)]
    extern "stdcall" fn foo2(x: u32) -> u32 {
        println!("original foo, x:{}", x);
        x * x
    }
    #[cfg(test)]
    unsafe extern "cdecl" fn on_foo2(
        reg: *mut Registers,
        old_func: usize,
        user_data: usize,
    ) -> usize {
        let old_func = std::mem::transmute::<usize, extern "stdcall" fn(u32) -> u32>(old_func);
        old_func((*reg).get_arg(1)) as usize + user_data
    }
    #[test]
    fn test_hook_function_stdcall() {
        assert_eq!(foo2(5), 25);
        let hooker = Hooker::new(
            foo2 as usize,
            HookType::Retn(4, on_foo2),
            CallbackOption::None,
            100,
            HookFlags::empty(),
        );
        let info = unsafe { hooker.hook().unwrap() };
        assert_eq!(foo2(5), 125);
        unsafe { info.unhook().unwrap() };
        assert_eq!(foo2(5), 25);
    }

    #[test]
    fn test_hook_function_cdecl_closure() {
        assert_eq!(foo(5), 25);
        let val_to_plus = 100;
        let new_foo = |reg: *mut Registers, old_func: usize| -> usize {
            let old_foo: fn(u32) -> u32 = unsafe { std::mem::transmute(old_func) };
            old_foo(unsafe { (*reg).get_arg(1) }) as usize + val_to_plus
        };
        let hook_point = unsafe {
            hook_closure_retn(foo as usize, 0, new_foo, CallbackOption::None, HookFlags::empty())
        }.unwrap();
        assert_eq!(foo(5), 125);
        drop(hook_point);
        assert_eq!(foo(5), 25);
    }
}
