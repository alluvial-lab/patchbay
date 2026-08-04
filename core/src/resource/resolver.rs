use patchbay_contracts::patchbay::TargetScope;

use crate::acceptance::{TargetBinding, TargetNotFound};

use super::{ResourceIdentity, ResourceRegistry};

pub(crate) fn resolve_resource(
    registry: &ResourceRegistry,
    target_scope: &TargetScope,
) -> Result<TargetBinding, TargetNotFound> {
    let identity = ResourceIdentity::try_from_scope(target_scope).map_err(|error| {
        TargetNotFound::NotFound {
            target: format!("invalid resource target: {error}"),
        }
    })?;
    if !registry.contains(&identity) {
        return Err(TargetNotFound::NotFound {
            target: format!("resource is not registered: {target_scope:?}"),
        });
    }
    Ok(TargetBinding::Resource(identity))
}
