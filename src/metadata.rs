use const_format::formatcp;

pub const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
pub const VERSION: &str = formatcp!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH);
