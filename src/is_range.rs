pub fn is_range(frames: &[i32]) -> bool {
    for i in 0..(frames.len() - 1) {
        let frame1 = frames[i];
        let frame2 = frames[i + 1];

        if (frame1 - frame2).abs() != 1 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::is_range::is_range;

    #[test]
    fn returns_true_rev() {
        assert!(is_range(&[4, 3, 2, 1]))
    }
    #[test]
    fn returns_false() {
        assert!(!is_range(&[1, 2, 3, 5, 6]))
    }
    #[test]
    fn returns_true() {
        assert!(is_range(&[1, 2, 3]))
    }
}
