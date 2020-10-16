//! Factory-programmed calibration data
//!
//! The STM32L0 contains a few read-only registers with factory calibration
//! data that are written during production.

macro_rules! define_ptr_type {
    ($name: ident, $ptr: expr) => {
        impl $name {
            fn ptr() -> *const Self {
                $ptr as *const _
            }

            /// Returns a wrapped reference to the value in flash memory
            pub fn get() -> &'static Self {
                unsafe { &*Self::ptr() }
            }
        }
    };
}

/// Internal voltage reference calibration data (VREFINT_CAL)
///
/// The internal voltage reference provides a stable (bandgap) voltage output
/// for the ADC and Comparators. The precise voltage of V_REFINT is
/// individually measured for each part by ST during production test and stored
/// in the system memory area. It can be accessed through this struct.
#[derive(Debug)]
#[repr(C)]
pub struct VrefintCal(u16);
define_ptr_type!(VrefintCal, 0x1FF8_0078);
impl VrefintCal {
    /// Read calibration value (VREFINT_CAL), acquired at temperature of 25 °C and V_DDA = 3 V.
    pub fn read(&self) -> u16 {
        self.0
    }
}

/// Temperature sensor calibration data acquired at 30 °C (TS_CAL1)
///
/// The temperature sensor (T_SENSE) generates a voltage (V_SENSE) that varies
/// linearly with temperature.
///
/// The sensor provides good linearity, but it has to be calibrated to obtain
/// good overall accuracy of the temperature measurement. As the offset of the
/// temperature sensor varies from chip to chip due to process variation, the
/// uncalibrated internal temperature sensor is suitable for applications that
/// detect temperature changes only.
///
/// To improve the accuracy of the temperature sensor measurement, each device
/// is individually factory-calibrated by ST. The temperature sensor factory
/// calibration data are stored by ST in the system memory area and can be
/// accessed through this struct.
#[derive(Debug)]
#[repr(C)]
pub struct VtempCal30(u16);
define_ptr_type!(VtempCal30, 0x1FF8_007A);
impl VtempCal30 {
    /// Read calibration value (TS_CAL1), acquired at temperature of 30 °C and V_DDA = 3 V.
    pub fn read(&self) -> u16 {
        self.0
    }
}

/// Temperature sensor calibration data acquired at 130 °C (TS_CAL2)
///
/// The temperature sensor (T_SENSE) generates a voltage (V_SENSE) that varies
/// linearly with temperature.
///
/// The sensor provides good linearity, but it has to be calibrated to obtain
/// good overall accuracy of the temperature measurement. As the offset of the
/// temperature sensor varies from chip to chip due to process variation, the
/// uncalibrated internal temperature sensor is suitable for applications that
/// detect temperature changes only.
///
/// To improve the accuracy of the temperature sensor measurement, each device
/// is individually factory-calibrated by ST. The temperature sensor factory
/// calibration data are stored by ST in the system memory area and can be
/// accessed through this struct.
#[derive(Debug)]
#[repr(C)]
pub struct VtempCal130(u16);
define_ptr_type!(VtempCal130, 0x1FF8_007E);
impl VtempCal130 {
    /// Read calibration value (TS_CAL2), acquired at temperature of 130 °C and V_DDA = 3 V.
    pub fn read(&self) -> u16 {
        self.0
    }
}
