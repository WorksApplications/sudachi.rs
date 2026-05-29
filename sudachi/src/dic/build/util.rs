/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::time::{SystemTime, UNIX_EPOCH};

pub fn default_signature(time: SystemTime, comment: &str) -> String {
    let secs = match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as libc::time_t,
        Err(e) => -(e.duration().as_secs() as libc::time_t),
    };
    let tm = local_tm(secs).unwrap_or_else(|| utc_tm(secs));
    let hash = java_string_hash(comment);
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}-{:08x}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        hash as u32
    )
}

pub fn java_string_hash(data: &str) -> i32 {
    data.encode_utf16()
        .fold(0i32, |acc, ch| acc.wrapping_mul(31).wrapping_add(ch as i32))
}

#[cfg(unix)]
pub fn local_tm(secs: libc::time_t) -> Option<libc::tm> {
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    let input = secs;
    let result = unsafe { libc::localtime_r(&input, out.as_mut_ptr()) };
    if result.is_null() {
        None
    } else {
        Some(unsafe { out.assume_init() })
    }
}

#[cfg(windows)]
pub fn local_tm(secs: libc::time_t) -> Option<libc::tm> {
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    let input = secs;
    let result = unsafe { libc::localtime_s(out.as_mut_ptr(), &input) };
    if result != 0 {
        None
    } else {
        Some(unsafe { out.assume_init() })
    }
}

#[cfg(unix)]
pub fn utc_tm(secs: libc::time_t) -> libc::tm {
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    let input = secs;
    let result = unsafe { libc::gmtime_r(&input, out.as_mut_ptr()) };
    if result.is_null() {
        zero_tm()
    } else {
        unsafe { out.assume_init() }
    }
}

#[cfg(windows)]
pub fn utc_tm(secs: libc::time_t) -> libc::tm {
    let mut out = std::mem::MaybeUninit::<libc::tm>::uninit();
    let input = secs;
    let result = unsafe { libc::gmtime_s(out.as_mut_ptr(), &input) };
    if result != 0 {
        zero_tm()
    } else {
        unsafe { out.assume_init() }
    }
}

fn zero_tm() -> libc::tm {
    unsafe { std::mem::zeroed() }
}
