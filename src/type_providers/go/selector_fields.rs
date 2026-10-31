//! Range the owner-local prefix instead of scanning all owners at every hop.
use super::GoTypeData;
use crate::resolution::GoOwnerIdentity;
use std::collections::BTreeSet;

pub(super) fn names(data: &GoTypeData, owner: &GoOwnerIdentity) -> BTreeSet<String> {
    #[cfg(test)]
    if crate::go_type_alias::import_memo::parallel::serial_reference_enabled() {
        return reference(data, owner);
    }
    data.field_types
        .range((owner.clone(), String::new())..)
        .take_while(|((field_owner, _), _)| field_owner == owner)
        .map(|((_, field), _)| field.clone())
        .chain(
            data.struct_embeds
                .get(owner)
                .into_iter()
                .flatten()
                .map(|embedded| embedded.name.clone()),
        )
        .collect()
}

#[cfg(test)]
fn reference(data: &GoTypeData, owner: &GoOwnerIdentity) -> BTreeSet<String> {
    data.field_types
        .keys()
        .filter(|(field_owner, _)| field_owner == owner)
        .map(|(_, field)| field.clone())
        .chain(
            data.struct_embeds
                .get(owner)
                .into_iter()
                .flatten()
                .map(|embedded| embedded.name.clone()),
        )
        .collect()
}

#[cfg(test)]
mod tests;
