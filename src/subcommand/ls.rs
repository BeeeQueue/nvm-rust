use std::borrow::Borrow;
use std::collections::HashSet;

use clap::ArgMatches;
use semver::VersionReq;

use crate::node_version::{NodeVersion, OnlineNodeVersion};
use crate::subcommand::Subcommand;

pub struct Ls {
    lts_version_strings: Vec<&'static str>,
    lts_version_reqs: Vec<VersionReq>,
}

impl Ls {
    pub fn init() -> Self {
        let strings = vec![">=10, <11", ">=12, <13", ">=14, <15"];

        Self {
            lts_version_strings: strings.to_owned(),
            lts_version_reqs: strings
                .iter()
                .map(|range| VersionReq::parse(range).unwrap())
                .collect(),
        }
    }

    pub fn validate_filter(value: &str) -> Result<VersionReq, String> {
        match value {
            val if (val.to_lowercase() == "lts") => Result::Ok(VersionReq::any()),
            val => VersionReq::parse(val).map_err(|_| String::from("Invalid version.")),
        }
    }

    fn fetch_versions() -> Result<Vec<OnlineNodeVersion>, String> {
        let response = reqwest::blocking::get("https://nodejs.org/dist/index.json");

        if response.is_err() {
            return Result::Err(response.unwrap_err().to_string());
        }

        let body = response.unwrap().text().unwrap();

        serde_json::from_str(body.borrow()).map_err(|err| {
            println!("{}", err);
            err.to_string()
        })
    }

    fn filter_major_versions(versions: Vec<OnlineNodeVersion>) -> Vec<OnlineNodeVersion> {
        let mut found_major_versions: HashSet<u64> = HashSet::new();

        versions
            .into_iter()
            .filter(|version| {
                let version = version.version();
                let major = version.major;

                if found_major_versions.contains(major.borrow()) {
                    return false;
                }

                found_major_versions.insert(major.clone());

                true
            })
            .collect()
    }
}

impl Subcommand for Ls {
    fn run(self, matches: &ArgMatches) -> Result<(), String> {
        let versions = Self::fetch_versions();

        if versions.is_err() {
            return Result::Err("versions error: ".to_owned() + &versions.unwrap_err());
        }

        let versions = versions.unwrap();
        let versions = Self::filter_major_versions(versions);
        let versions_str = versions
            .into_iter()
            .map(|version| format!("{:15}{}", version.version(), version.release_date))
            .collect::<Vec<String>>()
            .join("\n");

        let output_str = format!(
            "
Available for download:
{}

Specify a version range to show more results.
e.g. `nvm ls 12`
",
            versions_str
        );

        println!("{}", output_str.trim());

        Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    mod filter_major_versions {
        use std::borrow::Borrow;
        use std::fs;

        use crate::subcommand::ls::Ls;

        use super::super::OnlineNodeVersion;

        #[test]
        fn filters_correctly() {
            let test_data = fs::read_to_string("test-data/node-versions.json").unwrap();
            let test_data: Vec<OnlineNodeVersion> =
                serde_json::from_str(test_data.borrow()).unwrap();

            assert_eq!(
                Ls::filter_major_versions(test_data),
                vec![
                    OnlineNodeVersion::new(
                        String::from("14.6.0"),
                        String::from("2020-07-15"),
                        vec![]
                    ),
                    OnlineNodeVersion::new(
                        String::from("13.14.0"),
                        String::from("2020-04-28"),
                        vec![]
                    ),
                    OnlineNodeVersion::new(
                        String::from("12.18.3"),
                        String::from("2020-07-22"),
                        vec![]
                    ),
                    OnlineNodeVersion::new(
                        String::from("11.15.0"),
                        String::from("2019-04-30"),
                        vec![]
                    ),
                ]
            );
        }
    }
}
