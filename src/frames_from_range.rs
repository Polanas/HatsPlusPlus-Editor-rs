pub fn frames_from_range(start: i32, end: i32) -> Vec<i32> {
    let min_frame = i32::min(start, end);
    let max_frame = i32::max(start, end);
    let range = min_frame..=max_frame;
    if start > end {
        return range.rev().collect();
    }

    range.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_range() {
        assert_eq!(vec![1, 2, 3, 4, 5], frames_from_range(1, 5));
    }

    #[test]
    fn reverse_range() {
        assert_eq!(vec![6, 5, 4, 3], frames_from_range(6, 3));
    }

    #[test]
    fn small_range() {
        assert_eq!(vec![1], frames_from_range(1, 1));
    }
}
