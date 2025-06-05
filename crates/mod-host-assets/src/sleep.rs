use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};

/// Uses [`timeBeginPeriod`] to increase the resolution of the scheduler
/// for the process for the duration of the closure.
/// 
/// https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timebeginperiod
pub fn with_precise_sleep<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    unsafe {
        timeBeginPeriod(1);
    }

    let result = f();

    unsafe {
        timeEndPeriod(1);
    }

    result
}
