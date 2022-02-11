//=============================
// BVE ATS Plug-in Header File
//
//             Rock_On, mackoy
//=============================

// #ifdef ATS_EXPORTS
// #define ATS_API __declspec(dllexport)
// #else
// #define ATS_API __declspec(dllimport)
// #endif

#![allow(dead_code)]

use std::os::raw::*;

// ATS Plug-in Version
pub const ATS_VERSION: c_int = 0x0002_0000;

// ATS Keys
pub const ATS_KEY_S: c_int = 0; // S Key
pub const ATS_KEY_A1: c_int = 1; // A1 Key
pub const ATS_KEY_A2: c_int = 2; // A2 Key
pub const ATS_KEY_B1: c_int = 3; // B1 Key
pub const ATS_KEY_B2: c_int = 4; // B2 Key
pub const ATS_KEY_C1: c_int = 5; // C1 Key
pub const ATS_KEY_C2: c_int = 6; // C2 Key
pub const ATS_KEY_D: c_int = 7; // D Key
pub const ATS_KEY_E: c_int = 8; // R Key
pub const ATS_KEY_F: c_int = 9; // F Key
pub const ATS_KEY_G: c_int = 10; // G Key
pub const ATS_KEY_H: c_int = 11; // H Key
pub const ATS_KEY_I: c_int = 12; // I Key
pub const ATS_KEY_J: c_int = 13; // J Key
pub const ATS_KEY_K: c_int = 14; // K Key
pub const ATS_KEY_L: c_int = 15; // L Key

// Initial Position of Handle
pub const ATS_INIT_REMOVED: c_int = 2; // Handle Removed
pub const ATS_INIT_EMG: c_int = 1; // Emergency Brake
pub const ATS_INIT_SVC: c_int = 0; // Service Brake

// Sound Control Instruction
pub const ATS_SOUND_STOP: c_int = -10000; // Stop
pub const ATS_SOUND_PLAY: c_int = 1; // Play Once
pub const ATS_SOUND_PLAYLOOPING: c_int = 0; // Play Repeatedly
pub const ATS_SOUND_CONTINUE: c_int = 2; // Continue

// Type of Horn
pub const ATS_HORN_PRIMARY: c_int = 0; // Horn 1
pub const ATS_HORN_SECONDARY: c_int = 1; // Horn 2
pub const ATS_HORN_MUSIC: c_int = 2; // Music Horn

// Constant Speed Control Instruction
pub const ATS_CONSTANTSPEED_CONTINUE: c_int = 0; // Continue
pub const ATS_CONSTANTSPEED_ENABLE: c_int = 1; // Enable
pub const ATS_CONSTANTSPEED_DISABLE: c_int = 2; // Disable

// Vehicle Specification
#[repr(C)]
pub struct AtsVehicleSpec {
    pub brake_notches: c_int, // Number of Brake Notches
    pub power_notches: c_int, // Number of Power Notches
    pub ats_notch: c_int,     // ATS Cancel Notch
    pub b67_notch: c_int,     // 80% Brake (67 degree)
    pub cars: c_int,          // Number of Cars
}

// State Quantity of Vehicle
#[repr(C)]
pub struct AtsVehicleState {
    pub location: c_double,    // Train Position (Z-axis) (m)
    pub speed: c_float,        // Train Speed (km/h)
    pub time: c_int,           // Time (ms)
    pub bc_pressure: c_float,  // Pressure of Brake Cylinder (Pa)
    pub mr_pressure: c_float,  // Pressure of MR (Pa)
    pub er_pressure: c_float,  // Pressure of ER (Pa)
    pub bp_pressure: c_float,  // Pressure of BP (Pa)
    pub sap_pressure: c_float, // Pressure of SAP (Pa)
    pub current: c_float,      // Current (A)
}

// Received Data from Beacon
#[repr(C)]
pub struct AtsBeaconData {
    pub beacon_type: c_int, // Type of Beacon
    pub signal: c_int,      // Signal of Connected Section
    pub distance: c_float,  // Distance to Connected Section (m)
    pub optional: c_int,    // Optional Data
}

// Train Operation Instruction
#[repr(C)]
pub struct AtsHandles {
    pub brake: c_int,          // Brake Notch
    pub power: c_int,          // Power Notch
    pub reverser: c_int,       // Reverser Position
    pub constant_speed: c_int, // Constant Speed Control
}
