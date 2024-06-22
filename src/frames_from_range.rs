use crate::animations::Frame;

pub fn frames_from_range(start: i32, end: i32) -> Vec<Frame> {
    let min_frame = i32::min(start, end);
    let max_frame = i32::max(start, end);
    let range = min_frame..=max_frame;
    if start > end {
        return range.rev().map(|f| f.into()).collect();
    }

    range.map(|f| f.into()).collect()
}
