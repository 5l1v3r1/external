use std::{mem, ptr};
use std::ffi::OsString;
use std::os::windows::ffi::{OsStringExt};
use crate::winapi::*;
use crate::process::{ProcessId, ProcessRights};
use crate::thread::Thread;
use crate::error::ErrorCode;
use crate::{Result, IntoInner, FromInner};

/// Process handle.
#[derive(Debug)]
pub struct Process(HANDLE);
impl_inner!(Process: HANDLE);
impl Process {
	/// Get the current process.
	pub fn current() -> Process {
		Process(unsafe { GetCurrentProcess() })
	}
	/// Attach to a process by id and given rights.
	pub fn attach(pid: ProcessId, rights: ProcessRights) -> Result<Process> {
		// FIXME! What about handle inheritance?
		let handle = unsafe { OpenProcess(rights.into_inner(), TRUE, pid.into_inner()) };
		if handle.is_null() {
			Err(ErrorCode::last())
		}
		else {
			Ok(Process(handle))
		}
	}
	/// Get the id for this process.
	pub fn pid(&self) -> Result<ProcessId> {
		let pid = unsafe { GetProcessId(self.0) };
		if pid != 0 {
			Ok(ProcessId(pid))
		}
		else {
			Err(ErrorCode::last())
		}
	}
	/// Get the exit code for the process, `None` if the process is still running.
	pub fn exit_code(&self) -> Result<Option<DWORD>> {
		unsafe {
			let mut code: DWORD = mem::uninitialized();
			if GetExitCodeProcess(self.0, &mut code) != FALSE {
				Ok(if code == 259/*STILL_ACTIVE*/ { None } else { Some(code) })
			}
			else {
				Err(ErrorCode::last())
			}
		}
	}
	/// Wait for the process to finish.
	///
	/// See [WaitForSingleObject](https://msdn.microsoft.com/en-us/library/windows/desktop/ms687032.aspx) for more information.
	pub fn wait(&self, milis: DWORD) -> Result<DWORD> {
		unsafe {
			let result = WaitForSingleObject(self.0, milis);
			if result == WAIT_FAILED {
				Err(ErrorCode::last())
			}
			else {
				Ok(result)
			}
		}
	}
	pub fn create_thread(&self, start_address: usize, parameter: usize) -> Result<Thread> {
		unsafe {
			let handle = CreateRemoteThread(self.0, ptr::null_mut(), 0, mem::transmute(start_address), parameter as LPVOID, 0, ptr::null_mut());
			if handle.is_null() {
				Err(ErrorCode::last())
			}
			else {
				Ok(Thread::from_inner(handle))
			}
		}
	}
	pub fn full_image_name_wide<'a>(&self, buffer: &'a mut [u16]) -> Result<&'a mut [u16]> {
		unsafe {
			let mut size = buffer.len() as DWORD;
			if QueryFullProcessImageNameW(self.0, 0, buffer.as_mut_ptr(), &mut size) != FALSE {
				Ok(buffer.get_unchecked_mut(..size as usize))
			}
			else {
				Err(ErrorCode::last())
			}
		}
	}
	/// Get the full name of the executable for this process.
	pub fn full_image_name(&self) -> Result<OsString> {
		let mut buffer: [WCHAR; 0x400] = unsafe { mem::uninitialized() };
		self.full_image_name_wide(&mut buffer)
			.map(|path| OsString::from_wide(path))
	}
	pub fn get_mapped_file_name_wide<'a>(&self, address: usize, buffer: &'a mut [u16]) -> Result<&'a mut [u16]> {
		unsafe {
			let size = GetMappedFileNameW(self.0, address as LPVOID, buffer.as_mut_ptr(), buffer.len() as DWORD);
			if size != 0 {
				Ok(buffer.get_unchecked_mut(..size as usize))
			}
			else {
				Err(ErrorCode::last())
			}
		}
	}
}
impl Clone for Process {
	fn clone(&self) -> Process {
		Process(unsafe {
			let current = GetCurrentProcess();
			let mut new: HANDLE = mem::uninitialized();
			// What about all these options? inherit handles?
			let result = DuplicateHandle(current, self.0, current, &mut new, 0, FALSE, DUPLICATE_SAME_ACCESS);
			// Can't report error, should this ever fail?
			assert!(result != FALSE, "duplicate handle error: {}", ErrorCode::last());
			new
		})
	}
}
impl Drop for Process {
	fn drop(&mut self) {
		unsafe { CloseHandle(self.0); }
	}
}
