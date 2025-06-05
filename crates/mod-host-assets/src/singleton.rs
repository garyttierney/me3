use std::{collections::HashMap, ptr::NonNull, time};

use crate::sleep::with_precise_sleep;

pub fn poll_map(
    freq: time::Duration,
    timeout: time::Duration,
) -> Option<&'static HashMap<String, NonNull<*mut u8>>> {
    with_precise_sleep(|| {
        let map = from_singleton::map();

        let start = time::Instant::now();

        while time::Instant::now().checked_duration_since(start).unwrap() < timeout {
            if map.iter().any(|(_, v)| unsafe { !v.read().is_null() }) {
                return Some(map);
            }

            std::thread::sleep(freq);
        }

        None
    })
}
