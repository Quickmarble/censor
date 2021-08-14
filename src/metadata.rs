use const_format::formatcp;

pub const VERSION_MAJOR: u32 = 0;
pub const VERSION_MINOR: u32 = 2;
pub const VERSION_PATCH: u32 = 3;
pub const VERSION: &str = formatcp!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH);
