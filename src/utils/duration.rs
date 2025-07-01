pub trait DurExt {
    fn from_picos(picoseconds: u64) -> Self;
}

impl DurExt for std::time::Duration {
    fn from_picos(picoseconds: u64) -> Self {
        const PICOS_PER_NANO: u64 = 1000;
        let nanos = picoseconds / PICOS_PER_NANO;

        std::time::Duration::new(nanos / 1000000000, (nanos % 1000000000) as u32)
    }
}
