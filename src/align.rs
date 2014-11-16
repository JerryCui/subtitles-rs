//! Align two subtitle files.

use srt::{Subtitle, SubtitleFile};
use merge::merge_subtitles;

// How well do two subtitles match each other, going solely by the time?
#[deriving(PartialEq)]
enum MatchQuality {
    NoMatch,          // More than two seconds away.
    Nearby(f32),      // 0.0 <= seconds <= 2.0
    Overlap(f32)      // 0.0 < seconds
}

impl PartialOrd for MatchQuality {
    fn partial_cmp(&self, other: &MatchQuality) -> Option<Ordering> {
        match (self, other) {
            (&Overlap(v1), &Overlap(v2)) => v1.partial_cmp(&v2),
            (&Nearby(v1), &Nearby(v2)) =>
                v1.partial_cmp(&v2).map(|c| c.reverse()),
            (&NoMatch, &NoMatch) => Some(Equal),
            (&Overlap(_), _) => Some(Greater),
            (_, &Overlap(_)) => Some(Less),
            (&Nearby(_), _) => Some(Greater),
            (_, &Nearby(_)) => Some(Less)
        }
    }
}

// Calculate the match quality of two subtitles.
fn match_quality(sub1: &Subtitle, sub2: &Subtitle) -> MatchQuality {
    assert!(sub1.begin < sub1.end);
    assert!(sub2.begin < sub2.end);
    if sub1.end + 2.0 < sub2.begin || sub2.end + 2.0 < sub1.begin {
        NoMatch
    } else if sub1.end <= sub2.begin {
        let distance = (sub2.begin - sub1.end).abs();
        assert!(0.0 <= distance && distance <= 2.0);
        Nearby(distance)
    } else if sub2.end <= sub1.begin {
        let distance = (sub1.begin - sub2.end).abs();
        assert!(0.0 <= distance && distance <= 2.0);
        Nearby(distance)
    } else {
        let overlap = sub1.end.min(sub2.end) - sub1.begin.max(sub2.begin);
        assert!(0.0 < overlap);
        Overlap(overlap)
    }
}

// Find the index of the best match for `sub` in `candidates`.
fn best_match(sub: &Subtitle, candidates: &Vec<Subtitle>) -> Option<uint> {
    let mut best: Option<(uint, MatchQuality)> = None;
    for (i, candidate) in candidates.iter().enumerate() {
        let mq = match_quality(sub, candidate);
        if mq == NoMatch { continue; }
        match best {
            None => { best = Some((i, mq)); }
            Some((_, old_mq)) if mq > old_mq => { best = Some((i, mq)); }
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

// Find the index of the best match each subtitle in `subs` in `candidates`.
fn best_matches(subs: &Vec<Subtitle>, candidates: &Vec<Subtitle>) ->
    Vec<Option<uint>>
{
    // We could be a lot more efficient about this if we wanted.
    subs.iter().map(|s| best_match(s, candidates)).collect()
}

// Make sure the subtitles are in-order and have sensible begin/end times.
//fn clean_subs(file: &SubtitleFile) -> Vec<Subtitle> {
//    // Remove subtitles with bogus begin/end times.
//    let result = file.subtitles.iter().filter(|s| s.begin < s.end).collect();
//    // Sort subtitles by begin time.
//    result.sort_by(|a, b| a.begin.cmp(b.begin));
//    result
//}

/// Alignment specification, showing how to match up the specified indices
/// in two subtitle files.
type Alignment = Vec<(Vec<uint>, Vec<uint>)>;

// Returns true if `items[i].is_some()` and the value is found in `group`.
// Returns false if `i` is out of bounds.
fn group_contains(group: &Vec<uint>, items: &Vec<Option<uint>>, i: uint) -> bool{
    if !(i < items.len()) { return false; }
    match items[i] {
        None => false,
        Some(v) => group.iter().position(|e| *e == v).is_some()
    }
}

/// Find a good way to align two subtitle files.
fn alignment(file1: &SubtitleFile, file2: &SubtitleFile) -> Alignment {  
    let (subs1, subs2) = (&file1.subtitles, &file2.subtitles);
    // assert!(subs1 && subs2 contain valid subs in ascending order)
    let matches1 = best_matches(subs1, subs2);
    let matches2 = best_matches(subs2, subs1);
    let mut alignment: Alignment = vec!();
    let mut i1 = 0;
    let mut i2 = 0;
    while i1 < subs1.len() && i2 < subs2.len() {
        debug!("subs1: {} matches {}, subs2: {} matches {}",
               i1, matches1[i1], i2, matches2[i2]);
        if subs1[i1].begin < subs2[i2].begin && matches1[i1] != Some(i2) {
            // Subs1 has an item which doesn't match subs2.
            debug!("unmatched: [{}], []", i1);
            alignment.push((vec!(i1), vec!()));
            i1 += 1;
        } else if subs2[i2].begin < subs1[i1].begin && matches2[i2] != Some(i1) {
            // Subs2 has an item which doesn't match subs1.
            debug!("unmatched: [], [{}]", i2);
            alignment.push((vec!(), vec!(i2)));
            i2 += 1;
        } else {
            // We have some matches, so let's gather them all together.
            assert!(matches1[i1] == Some(i2) || matches2[i2] == Some(i1));
            let mut matched1 = vec!(i1); i1 += 1;
            let mut matched2 = vec!(i2); i2 += 1;
            while group_contains(&matched1, &matches2, i2) ||
                  group_contains(&matched2, &matches1, i1) {
                if group_contains(&matched1, &matches2, i2) {
                    // i2 matches something in matched1, so add to matched2.
                    matched2.push(i2); i2 += 1;
                } else if group_contains(&matched2, &matches1, i1) {
                    // i1 matches something in matched2, so add to matched1.
                    matched1.push(i1); i1 += 1;
                }
                debug!("grouping: {}, {}", matched1, matched2);
            }
            alignment.push((matched1, matched2));
        }
    }
    alignment
}

#[test]
fn test_alignment() {
    // Load sample subtitles.
    let path_es = Path::new("fixtures/sample.es.srt");
    let srt_es = SubtitleFile::from_path(&path_es).unwrap();
    let path_en = Path::new("fixtures/sample.en.srt");
    let srt_en = SubtitleFile::from_path(&path_en).unwrap();

    let expected =
        vec!((vec!(0), vec!(0, 1)),
             (vec!(),  vec!(2)),
             (vec!(1), vec!(3)),
             (vec!(2), vec!(4)),
             (vec!(3), vec!(5, 6)),
             (vec!(4), vec!(7)));
    assert_eq!(expected, alignment(&srt_es, &srt_en));
}

/// Align two subtitle files, merging subtitles as necessary.
pub fn align_files(file1: &SubtitleFile, file2: &SubtitleFile)
                   -> Vec<(Option<Subtitle>, Option<Subtitle>)>
{
    fn merge(file: &SubtitleFile, indices: &[uint]) -> Option<Subtitle> {
        let mut subs = vec!();
        for &i in indices.iter() {
            subs.push(file.subtitles[i].clone())
        }
        merge_subtitles(subs.as_slice())
    }

    alignment(file1, file2).iter().map(|&(ref indices1, ref indices2)| {
        (merge(file1, indices1.as_slice()),
         merge(file2, indices2.as_slice()))
    }).collect()
}

// Clone a subtitle with the specified index, and wrap its lines with
// formatting.
fn clone_as(sub: &Subtitle, index: uint, before: &str, after: &str) -> Subtitle {
    let lines =
        sub.lines.iter().map(|l| format!("{}{}{}", before, l, after)).collect();
    Subtitle{index: index, begin: sub.begin, end: sub.end, lines: lines}
}

/// Combine two subtitle files into an aligned file.
pub fn combine_files(file1: &SubtitleFile, file2: &SubtitleFile)
                     -> SubtitleFile
{
    let subs = align_files(file1, file2).iter().enumerate().map(|(i, pair)| {
        match pair {
            &(None, None) => panic!("Shouldn't have empty alignment pair!"),
            &(Some(ref sub), None) => clone_as(sub, i+1, "<i>", "</i>"),
            &(None, Some(ref sub)) => clone_as(sub, i+1, "", ""),
            &(Some(ref sub1), Some(ref sub2)) => {
                let mut new = clone_as(sub1, i+1, "<i>", "</i>");
                let mut lines = sub2.lines.clone();
                lines.push_all(new.lines.as_slice());
                new.lines = lines;
                new
            }
        }
    }).collect();
    SubtitleFile{subtitles: subs}
}

#[test]
fn test_combine_files() {
    // Load sample subtitles.
    let path_es = Path::new("fixtures/sample.es.srt");
    let srt_es = SubtitleFile::from_path(&path_es).unwrap();
    let path_en = Path::new("fixtures/sample.en.srt");
    let srt_en = SubtitleFile::from_path(&path_en).unwrap();
    let path_combined = Path::new("fixtures/combined.srt");
    let expected = SubtitleFile::from_path(&path_combined).unwrap();
    assert_eq!(expected.to_string(),
               combine_files(&srt_es, &srt_en).to_string());
}