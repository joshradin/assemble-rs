use semver::VersionReq;
use url::Url;

pub struct CrateRegistry {
    url: Url
}

pub struct CrateDependency {
    crate_name: String,
    version: VersionReq,
}
