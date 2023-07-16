#![cfg(windows)]

use once_cell::sync::Lazy;
use std::ffi::CString;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Error;
use std::io::ErrorKind;
use std::os::raw::*;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use winapi::shared::minwindef::{BOOL, DWORD, HMODULE, LPVOID, TRUE};
use winapi::shared::ntdef::WCHAR;

mod ats_plugin;
use ats_plugin::*;

fn to_wide_string_with_null<S: AsRef<OsStr>>(s: S) -> Vec<WCHAR> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide_string = s.as_ref().encode_wide().collect::<Vec<_>>();
    wide_string.push(0);
    wide_string
}

#[derive(Debug)]
struct ModuleFileNameError;

#[derive(Debug)]
struct LoadError {
    path: PathBuf,
    error: Error,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to load file '{}': {}",
            self.path.to_string_lossy(),
            self.error
        )
    }
}

type LoadFn = unsafe extern "system" fn();
type DisposeFn = unsafe extern "system" fn();
// type GetPluginVersionFn = unsafe extern "system" fn() -> c_int;
type SetVehicleSpecFn = unsafe extern "system" fn(AtsVehicleSpec);
type InitializeFn = unsafe extern "system" fn(c_int);
type ElapseFn = unsafe extern "system" fn(AtsVehicleState, *mut c_int, *mut c_int) -> AtsHandles;
type SetPowerFn = unsafe extern "system" fn(c_int);
type SetBrakeFn = unsafe extern "system" fn(c_int);
type SetReverserFn = unsafe extern "system" fn(c_int);
type KeyDownFn = unsafe extern "system" fn(c_int);
type KeyUpFn = unsafe extern "system" fn(c_int);
type HornBlowFn = unsafe extern "system" fn(c_int);
type DoorOpenFn = unsafe extern "system" fn();
type DoorCloseFn = unsafe extern "system" fn();
type SetSignalFn = unsafe extern "system" fn(c_int);
type SetBeaconDataFn = unsafe extern "system" fn(AtsBeaconData);

struct ChildPlugin {
    handle: HMODULE,
    load: Option<LoadFn>,
    dispose: Option<DisposeFn>,
    // get_plugin_version: Option<GetPluginVersionFn>,
    set_vehicle_spec: Option<SetVehicleSpecFn>,
    initialize: Option<InitializeFn>,
    elapse: Option<ElapseFn>,
    set_power: Option<SetPowerFn>,
    set_brake: Option<SetBrakeFn>,
    set_reverser: Option<SetReverserFn>,
    key_down: Option<KeyDownFn>,
    key_up: Option<KeyUpFn>,
    horn_blow: Option<HornBlowFn>,
    door_open: Option<DoorOpenFn>,
    door_close: Option<DoorCloseFn>,
    set_signal: Option<SetSignalFn>,
    set_beacon_data: Option<SetBeaconDataFn>,
    last_input: Option<AtsHandles>,
}

// SAFETY: ChildPlugin is not Send by default because of the `handle` field,
// which contains a raw pointer. It is safe to make it Send because the handle
// is only used in `drop`, which is not called concurrently.
unsafe impl Send for ChildPlugin {}

impl Drop for ChildPlugin {
    fn drop(&mut self) {
        use winapi::um::libloaderapi::FreeLibrary;
        unsafe {
            FreeLibrary(self.handle);
        }
    }
}

macro_rules! load_function {
    ($handle:expr, $name:expr) => {{
        use winapi::um::libloaderapi::GetProcAddress;
        let name = CString::new($name).unwrap();
        let f = unsafe { GetProcAddress($handle, name.as_ptr()) };
        if f.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute(f) })
        }
    }};
}

impl ChildPlugin {
    fn from_handle(handle: HMODULE) -> Self {
        ChildPlugin {
            handle,
            load: load_function!(handle, "Load"),
            dispose: load_function!(handle, "Dispose"),
            // get_plugin_version: load_function!(handle, "GetPluginVersion"),
            set_vehicle_spec: load_function!(handle, "SetVehicleSpec"),
            initialize: load_function!(handle, "Initialize"),
            elapse: load_function!(handle, "Elapse"),
            set_power: load_function!(handle, "SetPower"),
            set_brake: load_function!(handle, "SetBrake"),
            set_reverser: load_function!(handle, "SetReverser"),
            key_down: load_function!(handle, "KeyDown"),
            key_up: load_function!(handle, "KeyUp"),
            horn_blow: load_function!(handle, "HornBlow"),
            door_open: load_function!(handle, "DoorOpen"),
            door_close: load_function!(handle, "DoorClose"),
            set_signal: load_function!(handle, "SetSignal"),
            set_beacon_data: load_function!(handle, "SetBeaconData"),
            last_input: None,
        }
    }

    fn load(path: &Path) -> Result<Self, LoadError> {
        use winapi::um::libloaderapi::LoadLibraryW;
        let os_path = to_wide_string_with_null(path);
        let module = unsafe { LoadLibraryW(os_path.as_ptr()) };
        if module.is_null() {
            Err(LoadError {
                path: path.to_path_buf(),
                error: Error::last_os_error(),
            })
        } else {
            Ok(Self::from_handle(module))
        }
    }

    fn input_and_elapse(
        &mut self,
        mut handles: AtsHandles,
        vehicle_state: AtsVehicleState,
        panel: *mut c_int,
        sound: *mut c_int,
    ) -> AtsHandles {
        if self.last_input.map(|h| h.power) != Some(handles.power) {
            if let Some(set_power) = self.set_power {
                unsafe {
                    set_power(handles.power);
                }
            }
        }
        if self.last_input.map(|h| h.brake) != Some(handles.brake) {
            if let Some(set_brake) = self.set_brake {
                unsafe {
                    set_brake(handles.brake);
                }
            }
        }
        if self.last_input.map(|h| h.reverser) != Some(handles.reverser) {
            if let Some(set_reverser) = self.set_reverser {
                unsafe {
                    set_reverser(handles.reverser);
                }
            }
        }
        self.last_input = Some(handles);

        if let Some(elapse) = self.elapse {
            let new_handles = unsafe { elapse(vehicle_state, panel, sound) };
            handles = AtsHandles {
                constant_speed: new_handles.constant_speed.max(handles.constant_speed),
                ..new_handles
            };
        }
        handles
    }
}

#[derive(Default)]
struct Multiplexer {
    path: PathBuf,
    children: Vec<ChildPlugin>,
    power_input: c_int,
    brake_input: c_int,
    reverser_input: c_int,
}

impl Multiplexer {
    fn load(&mut self) -> Result<(), LoadError> {
        let list_file_path = {
            let mut path = self.path.clone();
            path.set_extension("txt");
            path
        };
        let list = match std::fs::read_to_string(&list_file_path) {
            Ok(list) => list,
            Err(error) => {
                return Err(LoadError {
                    path: list_file_path,
                    error,
                })
            }
        };

        let directory_path = self.path.parent().unwrap();

        self.children.reserve_exact(list.lines().count());
        let last_error = list
            .lines()
            .map(|relative_path| {
                let dll_absolute_path = {
                    let mut path = directory_path.to_path_buf();
                    path.push(relative_path);
                    if !path.starts_with(directory_path) {
                        // The path must be relative to make the list portable.
                        return Err(LoadError {
                            path: PathBuf::from(relative_path),
                            error: Error::new(ErrorKind::Other, "Non-relative path rejected"),
                        });
                    }
                    path
                };

                let child = ChildPlugin::load(&dll_absolute_path)?;
                if let Some(load) = child.load {
                    unsafe {
                        load();
                    }
                }
                self.children.push(child);
                Ok(())
            })
            .filter_map(Result::err)
            .last();
        match last_error {
            None => Ok(()),
            Some(error) => Err(error),
        }
    }
}

static MULTIPLEXER: Lazy<Mutex<Multiplexer>> = Lazy::new(|| Mutex::new(Multiplexer::default()));

fn get_module_file_name(module: HMODULE) -> Result<PathBuf, ModuleFileNameError> {
    use winapi::um::libloaderapi::GetModuleFileNameW;

    const LEN: DWORD = 0x8000;
    let mut buf = std::iter::repeat(0).take(LEN as usize).collect::<Vec<_>>();
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), LEN) };
    if 0 < len && len < LEN {
        Ok(PathBuf::from(OsString::from_wide(&buf[..len as usize])))
    } else {
        Err(ModuleFileNameError)
    }
}

#[no_mangle]
#[allow(non_snake_case)]
extern "system" fn DllMain(dll_module: HMODULE, call_reason: DWORD, _reserved: LPVOID) -> BOOL {
    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_THREAD_ATTACH: DWORD = 2;
    const DLL_THREAD_DETACH: DWORD = 3;
    const DLL_PROCESS_DETACH: DWORD = 0;

    match call_reason {
        DLL_PROCESS_ATTACH => {
            let mut multiplexer = MULTIPLEXER.lock().unwrap();
            multiplexer.path = get_module_file_name(dll_module).unwrap();
        }
        DLL_THREAD_ATTACH => (),
        DLL_THREAD_DETACH => (),
        DLL_PROCESS_DETACH => (),
        _ => (),
    }

    TRUE
}

fn show_error_dialog(error: LoadError, main_path: &Path) {
    use winapi::um::winuser::{MessageBoxW, MB_ICONWARNING, MB_OK, MB_TASKMODAL};
    let window = std::ptr::null_mut();
    let text = to_wide_string_with_null(error.to_string());
    let caption =
        to_wide_string_with_null(main_path.file_name().unwrap_or_else(|| OsStr::new("Error")));
    let r#type = MB_OK | MB_ICONWARNING | MB_TASKMODAL;
    unsafe {
        MessageBoxW(window, text.as_ptr(), caption.as_ptr(), r#type);
    }
}

/// Called when this plug-in is loaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Load() {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    match multiplexer.load() {
        Ok(()) => (),
        Err(error) => show_error_dialog(error, &multiplexer.path),
    }
}

/// Called when this plug_in is unloaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Dispose() {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(dispose) = child.dispose {
            unsafe {
                dispose();
            }
        }
    }
    multiplexer.children.clear();
}

/// Returns the version numbers of ATS plug-in
#[no_mangle]
pub extern "system" fn GetPluginVersion() -> c_int {
    ATS_VERSION
}

/// Called when the train is loaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetVehicleSpec(vehicle_spec: AtsVehicleSpec) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(set_vehicle_spec) = child.set_vehicle_spec {
            unsafe {
                set_vehicle_spec(vehicle_spec);
            }
        }
    }
}

/// Called when the game is started
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Initialize(brake: c_int) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(initialize) = child.initialize {
            unsafe {
                initialize(brake);
            }
        }
    }
}

/// Called every frame
///
/// # Safety
///
/// This function is marked as `unsafe` because it accesses the arrays pointed to by the argument
/// pointers. It is the caller's responsibility to make sure the arrays have 256 elements each and
/// have been properly initialized.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Elapse(
    vehicle_state: AtsVehicleState,
    panel: *mut c_int,
    sound: *mut c_int,
) -> AtsHandles {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    let mut handles = AtsHandles {
        brake: multiplexer.brake_input,
        power: multiplexer.power_input,
        reverser: multiplexer.reverser_input,
        constant_speed: ATS_CONSTANTSPEED_CONTINUE,
    };

    for child in &mut multiplexer.children {
        handles = child.input_and_elapse(handles, vehicle_state, panel, sound);
    }

    handles
}

/// Called when the power is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetPower(notch: c_int) {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    multiplexer.power_input = notch;
}

/// Called when the brake is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBrake(notch: c_int) {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    multiplexer.brake_input = notch;
}

/// Called when the reverser is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetReverser(pos: c_int) {
    let mut multiplexer = MULTIPLEXER.lock().unwrap();
    multiplexer.reverser_input = pos;
}

/// Called when any ATS key is pressed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyDown(key_code: c_int) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(key_down) = child.key_down {
            unsafe {
                key_down(key_code);
            }
        }
    }
}

/// Called when any ATS key is released
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyUp(key_code: c_int) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(key_up) = child.key_up {
            unsafe {
                key_up(key_code);
            }
        }
    }
}

/// Called when the horn is used
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn HornBlow(horn_type: c_int) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(horn_blow) = child.horn_blow {
            unsafe {
                horn_blow(horn_type);
            }
        }
    }
}

/// Called when the door is opened
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorOpen() {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(door_open) = child.door_open {
            unsafe {
                door_open();
            }
        }
    }
}

/// Called when the door is closed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorClose() {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(door_close) = child.door_close {
            unsafe {
                door_close();
            }
        }
    }
}

/// Called when current signal is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetSignal(signal: c_int) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(set_signal) = child.set_signal {
            unsafe {
                set_signal(signal);
            }
        }
    }
}

/// Called when the beacon data is received
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBeaconData(beacon_data: AtsBeaconData) {
    let multiplexer = MULTIPLEXER.lock().unwrap();
    for child in &multiplexer.children {
        if let Some(set_beacon_data) = child.set_beacon_data {
            unsafe {
                set_beacon_data(beacon_data);
            }
        }
    }
}
