use vek::Extent2;

use crate::*;

mod ddnet_external;
mod teeworlds_external;

/// Returns `true` if all clients of that version should have that mapres shipped with their client.
pub fn is_external_name(name: &str, version: Version) -> bool {
    match version {
        Version::DDNet06 => ddnet_external::is_external_name(name),
        Version::Teeworlds07 => teeworlds_external::is_external_name(name),
    }
}

/// Returns the opaque table for external images.
/// None, if the given name is not a valid external image name, or if the image not square.
/// The latter restriction is taken from the client source, don't ask me.
pub fn external_opaque_table(name: &str, version: Version) -> Option<[[bool; 16]; 16]> {
    match version {
        Version::DDNet06 => ddnet_external::external_opaque_table(name),
        Version::Teeworlds07 => teeworlds_external::external_opaque_table(name),
    }
}

/// Returns the (width, height) of external images.
/// None, if the given name is not a valid external image name.
pub fn external_dimensions(name: &str, version: Version) -> Option<Extent2<u32>> {
    match version {
        Version::DDNet06 => ddnet_external::external_dimensions(name),
        Version::Teeworlds07 => teeworlds_external::external_dimensions(name),
    }
}
