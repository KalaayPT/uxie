pub mod binary;
pub mod hgss_json;
pub mod platinum_json;
#[cfg(test)]
mod tests;

pub use binary::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
pub use hgss_json::HgssEventJson;
pub use platinum_json::{
    BgEventJson, CoordEventJson, ObjectEventJson, PlatinumEventJson, WarpEventJson,
};

/// Build an `InvalidData` error describing a bad field value.
pub(crate) fn invalid_data(field: &str, message: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("invalid value for `{field}`: {message}"),
    )
}
