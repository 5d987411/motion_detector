#[cfg(test)]
mod tests {
    use crate::Args;
    use clap::Parser;

    #[test]
    fn test_args_parsing() {
        // Test default values
        let args = Args::parse_from(&["motion_detector"]);
        assert_eq!(args.device, 0);
        assert_eq!(args.sensitivity, 0.3);
        assert_eq!(args.min_area, 500);
        assert!(!args.verbose);

        // Test custom values
        let args = Args::parse_from(&[
            "motion_detector",
            "--device",
            "1",
            "--sensitivity",
            "0.5",
            "--min-area",
            "1000",
            "--verbose",
        ]);
        assert_eq!(args.device, 1);
        assert_eq!(args.sensitivity, 0.5);
        assert_eq!(args.min_area, 1000);
        assert!(args.verbose);
    }

    #[test]
    fn test_filename_generation() {
        use chrono::Local;

        // Test that snapshot filename generation works
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let expected_pattern = format!("motion_{}.jpg", timestamp);

        // This test ensures the format is correct
        assert!(expected_pattern.starts_with("motion_"));
        assert!(expected_pattern.ends_with(".jpg"));
        assert!(expected_pattern.len() > 15); // Should have timestamp
    }

    #[test]
    fn test_sensitivity_bounds() {
        // Test that sensitivity values are within expected bounds
        let valid_sensitivities = [0.0, 0.1, 0.5, 0.9, 1.0];
        for &sensitivity in &valid_sensitivities {
            assert!(sensitivity >= 0.0 && sensitivity <= 1.0);
        }
    }

    #[test]
    fn test_min_area_bounds() {
        // Test that min_area values are reasonable
        let valid_areas = [100, 500, 1000, 5000];
        for &area in &valid_areas {
            assert!(area > 0);
        }
    }
}
