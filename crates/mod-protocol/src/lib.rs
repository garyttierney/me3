pub mod dependency;
pub mod game;
pub mod mod_file;
pub mod native;
pub mod package;
pub mod profile;

pub use game::Game;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use expect_test::expect_file;

    use crate::profile::ModProfile;

    fn check(test_case_name: &str) {
        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let test_case = test_data_dir.join(test_case_name);
        let test_snapshot = test_data_dir.join(format!("{test_case_name}.expected"));

        let actual_profile = ModProfile::from_file(&test_case).expect("parse failure");
        let expected_profile = expect_file![test_snapshot];

        expected_profile.assert_debug_eq(&actual_profile);
    }

    #[test]
    fn v1_basic_config() {
        check("v1/basic_config.me3");
    }

    #[test]
    fn v1_advanced_config() {
        check("v1/advanced_config.me3");
    }

    #[test]
    fn v1_plural_packages_name() {
        check("v1/plural_packages.me3");
    }

    #[test]
    fn v1_singular_packages_name() {
        check("v1/singular_package.me3");
    }

    #[test]
    fn v2_basic_config() {
        check("v2/basic_config.me3");
    }

    #[test]
    fn v2_advanced_config() {
        check("v2/advanced_config.me3");
    }

    #[test]
    fn v2_merge_configs() {
        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/v2");

        let profile_a =
            ModProfile::from_file(test_data_dir.join("merge_config_a.me3")).expect("parse failure");
        let profile_b =
            ModProfile::from_file(test_data_dir.join("merge_config_b.me3")).expect("parse failure");

        let merged_profile = profile_a
            .try_merge(&profile_b)
            .expect("failed to merge profiles");
        let expected_profile = expect_file![test_data_dir.join("merge_config.me3.expected")];

        expected_profile.assert_debug_eq(&merged_profile);
    }
}
