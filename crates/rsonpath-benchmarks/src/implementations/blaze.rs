use crate::framework::implementation::Implementation;
use std::ffi::{CStr, CString, NulError};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::NonNull;
use thiserror::Error;

unsafe extern "C" {
    fn blaze_last_error() -> *const c_char;
    fn blaze_compile_schema(schema_path: *const c_char) -> *mut c_void;
    fn blaze_load_instance(instance_path: *const c_char) -> *mut c_void;
    fn blaze_validate_schema_loaded_instance(schema_handle: *mut c_void, instance_handle: *mut c_void) -> c_int;
    fn blaze_destroy_schema(handle: *mut c_void);
    fn blaze_destroy_instance(handle: *mut c_void);
}

pub struct Blaze;

pub struct BlazeSchema(NonNull<c_void>);
pub struct BlazeInstance(NonNull<c_void>);

impl Implementation for Blaze {
    type Query = BlazeSchema;

    type File = BlazeInstance;

    type Error = BlazeError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "blaze"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(Blaze)
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let path = CString::new(file_path)?;
        let handle = unsafe { blaze_load_instance(path.as_ptr()) };
        let handle = NonNull::new(handle).ok_or_else(BlazeError::native_error)?;

        Ok(BlazeInstance(handle))
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let path = CString::new(schema_file_path)?;
        let handle = unsafe { blaze_compile_schema(path.as_ptr()) };
        let handle = NonNull::new(handle).ok_or_else(BlazeError::native_error)?;

        Ok(BlazeSchema(handle))
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        let result = unsafe { blaze_validate_schema_loaded_instance(query.0.as_ptr(), file.0.as_ptr()) };

        match result {
            -1 => Err(BlazeError::native_error()),
            0 | 1 => Ok("[validated]"),
            other => Err(BlazeError::UnexpectedReturnCode(other)),
        }
    }
}

impl Drop for BlazeSchema {
    fn drop(&mut self) {
        unsafe {
            blaze_destroy_schema(self.0.as_ptr());
        }
    }
}

impl Drop for BlazeInstance {
    fn drop(&mut self) {
        unsafe {
            blaze_destroy_instance(self.0.as_ptr());
        }
    }
}

#[derive(Error, Debug)]
pub enum BlazeError {
    #[error("could not convert path to a C string: {0}")]
    PathContainsNul(#[from] NulError),
    #[error("native Blaze error: {0}")]
    NativeError(String),
    #[error("unexpected Blaze return code: {0}")]
    UnexpectedReturnCode(i32),
}

impl BlazeError {
    fn native_error() -> Self {
        let message = unsafe {
            let ptr = blaze_last_error();
            if ptr.is_null() {
                String::from("unknown Blaze error")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };

        BlazeError::NativeError(message)
    }
}
