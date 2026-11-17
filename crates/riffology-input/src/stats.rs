//! Percentiles and onset scoring, shared by the tests and the app's latency readout.

/// The value at a percentile (0 to 100) of the samples, by nearest rank, or None for no samples.
/// @param values - the samples, in any order
/// @param pct - the percentile
pub fn percentile(values: &[f64], pct: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = ((pct / 100.0) * v.len() as f64).ceil().max(1.0) as usize;
    Some(v[rank.min(v.len()) - 1])
}

/// Onset F1 the way mir_eval scores note onsets: an estimated onset matches a reference onset of the same rounded
/// pitch within 50 ms, each used once, matched to maximize the count.
/// @param refs - reference notes as (onset seconds, midi)
/// @param est - estimated notes as (onset seconds, midi)
pub fn onset_f1(refs: &[(f64, f64)], est: &[(f64, f64)]) -> f64 {
    if refs.is_empty() || est.is_empty() {
        return 0.0;
    }
    let adj: Vec<Vec<usize>> = refs
        .iter()
        .map(|r| {
            est.iter()
                .enumerate()
                .filter(|(_, e)| (e.0 - r.0).abs() <= 0.05 && e.1.round() == r.1.round())
                .map(|(j, _)| j)
                .collect()
        })
        .collect();
    let mut owner: Vec<Option<usize>> = vec![None; est.len()];
    let mut matched = 0;
    for i in 0..refs.len() {
        let mut seen = vec![false; est.len()];
        if augment(i, &adj, &mut owner, &mut seen) {
            matched += 1;
        }
    }
    let p = matched as f64 / est.len() as f64;
    let r = matched as f64 / refs.len() as f64;
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

/// One step of bipartite matching: finds an estimate for reference `i`, moving earlier matches when that helps.
/// @param i - the reference
/// @param adj - the estimates each reference may match
/// @param owner - the reference each estimate is matched to
/// @param seen - estimates visited in this search
fn augment(i: usize, adj: &[Vec<usize>], owner: &mut [Option<usize>], seen: &mut [bool]) -> bool {
    for &j in &adj[i] {
        if seen[j] {
            continue;
        }
        seen[j] = true;
        if owner[j].is_none() || augment(owner[j].unwrap(), adj, owner, seen) {
            owner[j] = Some(i);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_by_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        assert_eq!(percentile(&v, 50.0), Some(50.0));
        assert_eq!(percentile(&v, 99.0), Some(99.0));
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn onsets_match_by_pitch_within_fifty_milliseconds() {
        let refs = [(1.0, 60.0), (2.0, 64.0)];
        assert_eq!(onset_f1(&refs, &[(1.04, 60.0), (2.0, 64.0)]), 1.0);
        assert_eq!(onset_f1(&refs, &[(1.06, 60.0), (2.0, 65.0)]), 0.0);
        assert!((onset_f1(&refs, &[(1.0, 60.0)]) - 2.0 / 3.0).abs() < 1e-12);
    }
}
