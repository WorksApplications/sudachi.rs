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

//! Portable software-prefetch hint.

/// Hint the CPU to fetch the cache line containing `ptr`. Does not dereference
/// `ptr`, so any address is sound; callers must not rely on whether the hint is
/// honored. A no-op on unsupported targets.
#[inline(always)]
pub fn prefetch_l1<T>(ptr: *const T) {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: `_mm_prefetch` is a hint and never dereferences the pointer.
        unsafe {
            core::arch::x86_64::_mm_prefetch::<{ core::arch::x86_64::_MM_HINT_T0 }>(
                ptr as *const i8,
            );
        }
    }

    #[cfg(all(target_arch = "x86", target_feature = "sse"))]
    {
        // SAFETY: `_mm_prefetch` is a hint and never dereferences the pointer.
        unsafe {
            core::arch::x86::_mm_prefetch::<{ core::arch::x86::_MM_HINT_T0 }>(ptr as *const i8);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: `prfm` is a hint; it never faults and writes no memory.
        // `readonly` avoids a write barrier; no `pure` so it is not elided.
        unsafe {
            core::arch::asm!(
                "prfm pldl1keep, [{p}]",
                p = in(reg) ptr,
                options(nostack, preserves_flags, readonly),
            );
        }
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        all(target_arch = "x86", target_feature = "sse"),
        target_arch = "aarch64"
    )))]
    {
        let _ = ptr;
    }
}

#[cfg(test)]
mod tests {
    use super::prefetch_l1;

    #[test]
    fn prefetch_valid_pointer_is_noop_observable() {
        let data = [1u32, 2, 3, 4];
        prefetch_l1(data.as_ptr());
        prefetch_l1(unsafe { data.as_ptr().add(3) });
        assert_eq!(data, [1, 2, 3, 4]);
    }
}
