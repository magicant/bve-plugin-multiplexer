#![cfg(windows)]

use std::cell::RefCell;
use std::ffi::CString;
use std::ffi::OsString;
use std::os::raw::*;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;
use winapi::shared::minwindef::{BOOL, DWORD, HMODULE, LPVOID, TRUE};

mod ats_plugin;
use ats_plugin::*;

const ARRAY_LENGTH: usize = 256;

#[derive(Debug)]
struct LoadFailure;

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
    last_input: AtsHandles,
}

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
            last_input: AtsHandles {
                power: 0,
                brake: 0,
                reverser: 0,
                constant_speed: ATS_CONSTANTSPEED_CONTINUE,
            },
        }
    }

    fn load(path: &Path) -> Result<Self, LoadFailure> {
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::libloaderapi::LoadLibraryW;
        let mut path: Vec<_> = path.as_os_str().encode_wide().collect();
        path.push(0x0000);
        let module = unsafe { LoadLibraryW(path.as_ptr()) };
        if module.is_null() {
            Err(LoadFailure)
        } else {
            Ok(Self::from_handle(module))
        }
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

thread_local! {
    static MULTIPLEXER: RefCell<Multiplexer> = RefCell::new(Multiplexer::default());
}

fn get_module_file_name(module: HMODULE) -> Result<PathBuf, LoadFailure> {
    use winapi::um::libloaderapi::GetModuleFileNameW;

    const LEN: DWORD = 0x8000;
    let mut buf = std::iter::repeat(0).take(LEN as usize).collect::<Vec<_>>();
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), LEN) };
    if 0 < len && len < LEN {
        Ok(PathBuf::from(OsString::from_wide(&buf[..len as usize])))
    } else {
        Err(LoadFailure)
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
        DLL_PROCESS_ATTACH | DLL_THREAD_ATTACH => {
            MULTIPLEXER.with(|multiplexer| {
                let mut multiplexer = multiplexer.borrow_mut();
                multiplexer.path = get_module_file_name(dll_module).unwrap();
            });
        }
        DLL_THREAD_DETACH | DLL_PROCESS_DETACH => (),
        _ => (),
    }

    TRUE
}

/// Called when this plug-in is loaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Load() {
    //TODO Load child plugins
}

/// Called when this plug_in is unloaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Dispose() {
    MULTIPLEXER.with(|multiplexer| {
        let mut multiplexer = multiplexer.borrow_mut();
        for child in &multiplexer.children {
            if let Some(dispose) = child.dispose {
                unsafe {
                    dispose();
                }
            }
        }
        multiplexer.children.clear();
    })
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
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(set_vehicle_spec) = child.set_vehicle_spec {
                unsafe {
                    set_vehicle_spec(vehicle_spec);
                }
            }
        }
    })
}

/// Called when the game is started
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Initialize(brake: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(initialize) = child.initialize {
                unsafe {
                    initialize(brake);
                }
            }
        }
    })
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
    _vehicle_state: AtsVehicleState,
    p_panel: *mut c_int,
    p_sound: *mut c_int,
) -> AtsHandles {
    let _panel = std::slice::from_raw_parts_mut(p_panel, ARRAY_LENGTH);
    let _sound = std::slice::from_raw_parts_mut(p_sound, ARRAY_LENGTH);

    // TODO Call child plugins

    MULTIPLEXER.with(|multiplexer| {
        let multiplexer = multiplexer.borrow();
        AtsHandles {
            brake: multiplexer.brake_input,
            power: multiplexer.power_input,
            reverser: multiplexer.reverser_input,
            constant_speed: ATS_CONSTANTSPEED_CONTINUE,
        }
    })
}

/// Called when the power is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetPower(notch: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        let mut multiplexer = multiplexer.borrow_mut();
        multiplexer.power_input = notch;
    })
}

/// Called when the brake is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBrake(notch: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        let mut multiplexer = multiplexer.borrow_mut();
        multiplexer.brake_input = notch;
    })
}

/// Called when the reverser is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetReverser(pos: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        let mut multiplexer = multiplexer.borrow_mut();
        multiplexer.reverser_input = pos;
    })
}

/// Called when any ATS key is pressed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyDown(key_code: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(key_down) = child.key_down {
                unsafe {
                    key_down(key_code);
                }
            }
        }
    })
}

/// Called when any ATS key is released
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyUp(key_code: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(key_up) = child.key_up {
                unsafe {
                    key_up(key_code);
                }
            }
        }
    })
}

/// Called when the horn is used
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn HornBlow(horn_type: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(horn_blow) = child.horn_blow {
                unsafe {
                    horn_blow(horn_type);
                }
            }
        }
    })
}

/// Called when the door is opened
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorOpen() {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(door_open) = child.door_open {
                unsafe {
                    door_open();
                }
            }
        }
    })
}

/// Called when the door is closed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorClose() {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(door_close) = child.door_close {
                unsafe {
                    door_close();
                }
            }
        }
    })
}

/// Called when current signal is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetSignal(signal: c_int) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(set_signal) = child.set_signal {
                unsafe {
                    set_signal(signal);
                }
            }
        }
    })
}

/// Called when the beacon data is received
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBeaconData(beacon_data: AtsBeaconData) {
    MULTIPLEXER.with(|multiplexer| {
        for child in &multiplexer.borrow().children {
            if let Some(set_beacon_data) = child.set_beacon_data {
                unsafe {
                    set_beacon_data(beacon_data);
                }
            }
        }
    })
}
