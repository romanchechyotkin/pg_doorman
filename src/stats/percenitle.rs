pub fn percentile_of_sorted(sorted_samples: &[u64], pct: f64) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }
    if sorted_samples.len() == 1 {
        return sorted_samples[0];
    }
    let rank = (pct * sorted_samples.len() as f64 / 100.0).ceil() as usize;
    sorted_samples[rank - 1]
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_percentile() {
        let mut val = vec![0, 1, 2, 8, 9, 3, 4, 5, 6, 7];
        let times = val.as_mut_slice();
        times.sort();
        assert_eq!(percentile_of_sorted(times, 10.0), 0);
        assert_eq!(percentile_of_sorted(times, 50.0), 4);
        assert_eq!(percentile_of_sorted(times, 99.0), 9);
    }
}
