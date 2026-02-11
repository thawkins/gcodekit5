use libloading::Library;
use std::sync::Once;

pub(crate) static mut EPOXY_LIB: Option<Library> = None;
pub(crate) static mut GL_LIB: Option<Library> = None;
pub(crate) static EPOXY_INIT: Once = Once::new();

pub(crate) fn load_gl_func(name: &str) -> *const std::ffi::c_void {
    unsafe {
        EPOXY_INIT.call_once(|| {
            let lib = Library::new("libepoxy.so.0")
                .or_else(|_| Library::new("libepoxy.so"))
                .ok();
            EPOXY_LIB = lib;

            let gl_lib = Library::new("libGL.so.1")
                .or_else(|_| Library::new("libGL.so"))
                .ok();
            GL_LIB = gl_lib;
        });

        // Try epoxy first
        if let Some(lib) = (*std::ptr::addr_of!(EPOXY_LIB)).as_ref() {
            // Try epoxy_get_proc_addr
            if let Ok(get_proc_addr) = lib
                .get::<unsafe extern "C" fn(*const i8) -> *const std::ffi::c_void>(
                    b"epoxy_get_proc_addr",
                )
            {
                if let Ok(c_name) = std::ffi::CString::new(name) {
                    let ptr = get_proc_addr(c_name.as_ptr());
                    if !ptr.is_null() {
                        return ptr;
                    }
                }
            }
            // Fallback: try to load symbol directly from epoxy
            if let Ok(c_name) = std::ffi::CString::new(name) {
                if let Ok(sym) = lib.get::<*const std::ffi::c_void>(c_name.as_bytes()) {
                    return *sym;
                }
            }
        }

        // Try libGL as fallback
        if let Some(lib) = (*std::ptr::addr_of!(GL_LIB)).as_ref() {
            if let Ok(c_name) = std::ffi::CString::new(name) {
                if let Ok(sym) = lib.get::<*const std::ffi::c_void>(c_name.as_bytes()) {
                    return *sym;
                }
            }
        }

        std::ptr::null()
    }
}
