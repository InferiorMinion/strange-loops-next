/// splitmix64: a fixed seed gives the same history on every machine.
pub(super) struct Rng(pub(super) u64);
impl Rng {
    pub(super) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A number below `n`.
    pub(super) fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("n fits")).expect("below n")
    }
    pub(super) fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

pub(super) fn emitted(step: usize) -> String {
    format!(
        "2026-11-14T{:02}:{:02}:{:02}Z",
        step / 3600,
        step / 60 % 60,
        step % 60
    )
}
