#![cfg(windows)]

use std::cell::Cell;
use std::ffi::CString;
use std::os::raw::*;
use std::path::Path;
use winapi::shared::minwindef::{BOOL, DWORD, HMODULE, LPVOID, TRUE};

mod ats_plugin;
use ats_plugin::*;

const ARRAY_LENGTH: usize = 256;

struct LoadFailure;

type LoadFn = unsafe extern "system" fn();
type DisposeFn = unsafe extern "system" fn();
type GetPluginVersionFn = unsafe extern "system" fn() -> c_int;
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
    get_plugin_version: Option<GetPluginVersionFn>,
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
            get_plugin_version: load_function!(handle, "GetPluginVersion"),
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

thread_local! {
    static POWER: Cell<c_int> = Cell::new(0);
    static BRAKE: Cell<c_int> = Cell::new(0);
    static REVERSER: Cell<c_int> = Cell::new(0);
}

#[no_mangle]
#[allow(non_snake_case)]
extern "system" fn DllMain(_dll_module: HMODULE, call_reason: DWORD, _reserved: LPVOID) -> BOOL {
    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_THREAD_ATTACH: DWORD = 2;
    const DLL_THREAD_DETACH: DWORD = 3;
    const DLL_PROCESS_DETACH: DWORD = 0;

    match call_reason {
        DLL_PROCESS_ATTACH => (),
        DLL_THREAD_ATTACH => (),
        DLL_THREAD_DETACH => (),
        DLL_PROCESS_DETACH => (),
        _ => (),
    }

    TRUE
}

/// Called when this plug-in is loaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Load() {}

/// Called when this plug_in is unloaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Dispose() {}

/// Returns the version numbers of ATS plug-in
#[no_mangle]
pub extern "system" fn GetPluginVersion() -> c_int {
    ATS_VERSION
}

/// Called when the train is loaded
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetVehicleSpec(_vehicle_spec: AtsVehicleSpec) {}

/// Called when the game is started
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Initialize(_brake: c_int) {}

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

    AtsHandles {
        brake: BRAKE.with(Cell::get),
        power: POWER.with(Cell::get),
        reverser: REVERSER.with(Cell::get),
        constant_speed: ATS_CONSTANTSPEED_CONTINUE,
    }
}

/// Called when the power is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetPower(notch: c_int) {
    POWER.with(|power| {
        power.set(notch);
    });
}

/// Called when the brake is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBrake(notch: c_int) {
    BRAKE.with(|brake| {
        brake.set(notch);
    });
}

/// Called when the reverser is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetReverser(pos: c_int) {
    REVERSER.with(|reverser| {
        reverser.set(pos);
    });
}

/// Called when any ATS key is pressed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyDown(_ats_key_code: c_int) {}

/// Called when any ATS key is released
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn KeyUp(_ats_key_code: c_int) {}

/// Called when the horn is used
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn HornBlow(_horn_type: c_int) {}

/// Called when the door is opened
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorOpen() {}

/// Called when the door is closed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DoorClose() {}

/// Called when current signal is changed
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetSignal(_signal: c_int) {}

/// Called when the beacon data is received
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn SetBeaconData(_beacon_data: AtsBeaconData) {}
