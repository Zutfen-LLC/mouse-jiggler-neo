//! Tiny LCG for the random-timer feature.
//! Not cryptographic — just enough variation to make the jiggle interval
//! unpredictable. Seeded from GetTickCount64 at startup.

use windows::Win32::System::SystemInformation::GetTickCount64;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new() -> Self {
        let seed = unsafe { GetTickCount64() }.max(1);
        Self { state: seed }
    }

    /// Numerical Recipes 64-bit LCG.
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    /// Uniform integer in [lo, hi] inclusive. Assumes lo <= hi.
    pub fn range_inclusive(&mut self, lo: u32, hi: u32) -> u32 {
        let span = (hi - lo) as u64 + 1;
        let r = (self.next_u64() >> 32) % span;
        lo + r as u32
    }
}
