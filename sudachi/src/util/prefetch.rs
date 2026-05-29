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
//!
//! `prefetch_l1` asks the CPU to start pulling the cache line containing `ptr`
//! into the L1 data cache. It is only a *hint*: the hardware is free to ignore
//! it and it never faults, so passing a speculative (even out-of-bounds)
//! address is sound. This is what lets the trie matcher prefetch the next
//! symbol's node "even with not 100% accuracy" (see issue #117).

/// Issue a temporal L1 read-prefetch for the cache line containing `ptr`.
///
/// The hint never dereferences `ptr`, so it is safe for any address. On
/// architectures without a stable prefetch primitive this compiles to nothing.
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

    #[cfg(target_arch = "x86")]
    {
        // SAFETY: `_mm_prefetch` is a hint and never dereferences the pointer.
        unsafe {
            core::arch::x86::_mm_prefetch::<{ core::arch::x86::_MM_HINT_T0 }>(ptr as *const i8);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: `prfm` is a hint instruction; it does not fault on a bad
        // address and writes no memory. `readonly` keeps it from inserting a
        // write barrier in the hot loop; omitting `pure` keeps it from being
        // optimized away despite the unused result.
        unsafe {
            core::arch::asm!(
                "prfm pldl1keep, [{p}]",
                p = in(reg) ptr,
                options(nostack, preserves_flags, readonly),
            );
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    {
        // No stable prefetch primitive on this target; the hint is a no-op.
        let _ = ptr;
    }
}

#[cfg(test)]
mod tests {
    use super::prefetch_l1;

    #[test]
    fn prefetch_valid_pointer_is_noop_observable() {
        let data = [1u32, 2, 3, 4];
        // Prefetching a valid address must not change observable state.
        prefetch_l1(data.as_ptr());
        prefetch_l1(unsafe { data.as_ptr().add(3) });
        assert_eq!(data, [1, 2, 3, 4]);
    }
}
