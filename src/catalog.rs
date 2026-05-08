/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::config::{AllowedTypeError, AllowedTypeMatch, Config};
use crate::error::ErrorLocation;
use crate::path::Path;
use rustdoc_types::{Id, Item, Span};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, HashMap};

fn external_crate_of_type_name(type_name: &str) -> Option<&str> {
    type_name.split("::").next()
}

fn json_span_object(span: &Span) -> JsonValue {
    json!({
        "filename": span.filename.to_string_lossy(),
        "begin": [span.begin.0, span.begin.1],
        "end": [span.end.0, span.end.1],
    })
}

fn json_location_object(loc: Option<&Span>) -> JsonValue {
    loc.map(json_span_object).unwrap_or(JsonValue::Null)
}

fn id_string(id: &Id) -> String {
    format!("{}", id.0)
}

#[derive(Clone, Debug)]
pub struct DirectExternalUsage {
    pub type_name: String,
    pub external_crate: Option<String>,
    pub is_approved: bool,
    pub is_std: bool,
    pub what: String,
    pub in_what_type: String,
    pub location: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct TransitiveExternalUsage {
    pub type_name: String,
    pub external_crate: Option<String>,
    pub is_approved: bool,
    pub chain: Vec<String>,
    pub what: String,
    pub in_what_type: String,
    pub location: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub id: String,
    pub kind: &'static str,
    pub path: String,
    pub name: String,
    pub parent_path: Option<String>,
    pub location: Option<Span>,
}

#[derive(Default)]
pub struct ItemCatalog {
    enabled: bool,
    items: BTreeMap<String, ItemRecord>,
    direct: HashMap<String, Vec<DirectExternalUsage>>,
    transitive: HashMap<String, Vec<TransitiveExternalUsage>>,
}

impl ItemCatalog {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn record_item(&mut self, item: &Item, kind: &'static str, path: &Path) {
        if !self.enabled {
            return;
        }
        let id = id_string(&item.id);
        let path_str = path.to_string();
        let name = item.name.clone().unwrap_or_default();
        let parent_path = path.parent_path_string();
        self.items.entry(id.clone()).or_insert_with(|| ItemRecord {
            id,
            kind,
            path: path_str,
            name,
            parent_path,
            location: item.span.clone(),
        });
    }

    pub fn record_direct_external(
        &mut self,
        owner_id: &Id,
        usage: DirectExternalUsage,
    ) {
        if !self.enabled {
            return;
        }
        self.direct
            .entry(id_string(owner_id))
            .or_default()
            .push(usage);
    }

    pub fn record_transitive_external(
        &mut self,
        entry_struct_id: &Id,
        usage: TransitiveExternalUsage,
    ) {
        if !self.enabled {
            return;
        }
        self.transitive
            .entry(id_string(entry_struct_id))
            .or_default()
            .push(usage);
    }

    fn direct_usage_json(d: &DirectExternalUsage) -> JsonValue {
        json!({
            "type_name": d.type_name,
            "external_crate": d.external_crate,
            "is_approved": d.is_approved,
            "is_std": d.is_std,
            "what": d.what,
            "in_what_type": d.in_what_type,
            "location": json_location_object(d.location.as_ref()),
        })
    }

    fn transitive_usage_json(t: &TransitiveExternalUsage) -> JsonValue {
        json!({
            "type_name": t.type_name,
            "external_crate": t.external_crate,
            "is_approved": t.is_approved,
            "chain": t.chain,
            "what": t.what,
            "in_what_type": t.in_what_type,
            "location": json_location_object(t.location.as_ref()),
        })
    }

    pub fn to_json_value(&self) -> JsonValue {
        let mut items_out: Vec<JsonValue> = Vec::new();
        for (id, rec) in &self.items {
            let direct_usage = self.direct.get(id);
            let transitive_usage = self.transitive.get(id);
            let uses_external = direct_usage.map(|v| !v.is_empty()).unwrap_or(false)
                || transitive_usage.map(|v| !v.is_empty()).unwrap_or(false);
            let uses_unapproved_external = direct_usage
                .map(|v| v.iter().any(|d| !d.is_approved))
                .unwrap_or(false)
                || transitive_usage
                    .map(|v| v.iter().any(|t| !t.is_approved))
                    .unwrap_or(false);
            let direct = direct_usage
                .map(|v| v.iter().map(Self::direct_usage_json).collect::<Vec<_>>())
                .unwrap_or_default();
            let transitive = transitive_usage
                .map(|v| v.iter().map(Self::transitive_usage_json).collect::<Vec<_>>())
                .unwrap_or_default();
            items_out.push(json!({
                "id": rec.id,
                "kind": rec.kind,
                "path": rec.path,
                "name": rec.name,
                "parent_path": rec.parent_path,
                "location": json_location_object(rec.location.as_ref()),
                "external_usage": {
                    "uses_external": uses_external,
                    "uses_unapproved_external": uses_unapproved_external,
                    "direct_externals": direct,
                    "transitive_externals": transitive,
                },
            }));
        }
        let items_with_external_usage = items_out
            .iter()
            .filter(|v| {
                v.get("external_usage")
                    .and_then(|e| e.get("uses_external"))
                    .and_then(|b| b.as_bool())
                    == Some(true)
            })
            .count();
        let items_with_unapproved_external_usage = items_out
            .iter()
            .filter(|v| {
                v.get("external_usage")
                    .and_then(|e| e.get("uses_unapproved_external"))
                    .and_then(|b| b.as_bool())
                    == Some(true)
            })
            .count();
        json!({
            "summary": {
                "items_count": items_out.len(),
                "items_with_external_usage": items_with_external_usage,
                "items_with_unapproved_external_usage": items_with_unapproved_external_usage,
            },
            "items": items_out,
        })
    }
}

pub fn is_approved_for_type_name(config: &Config, root_crate_name: &str, type_name: &str) -> bool {
    matches!(
        config.allows_type(root_crate_name, type_name),
        Ok(AllowedTypeMatch::RootMatch)
            | Ok(AllowedTypeMatch::StandardLibrary(_))
            | Ok(AllowedTypeMatch::WildcardMatch(_))
    )
}

pub fn record_catalog_external_from_allow_result(
    catalog: &mut ItemCatalog,
    path: &Path,
    what: &ErrorLocation,
    type_name: &str,
    owner_id: &Id,
    allow_result: &Result<AllowedTypeMatch<'_>, AllowedTypeError<'_>>,
) {
    if !catalog.is_enabled() {
        return;
    }
    let in_what_type = path.to_string();
    let location = path.last_span().cloned();
    let what_str = what.to_string();
    let ext_crate = external_crate_of_type_name(type_name).map(|s| s.to_string());
    match allow_result {
        Ok(AllowedTypeMatch::RootMatch) => {}
        Ok(AllowedTypeMatch::StandardLibrary(_)) => {
            catalog.record_direct_external(
                owner_id,
                DirectExternalUsage {
                    type_name: type_name.to_string(),
                    external_crate: ext_crate,
                    is_approved: true,
                    is_std: true,
                    what: what_str,
                    in_what_type,
                    location,
                },
            );
        }
        Ok(AllowedTypeMatch::WildcardMatch(_)) => {
            catalog.record_direct_external(
                owner_id,
                DirectExternalUsage {
                    type_name: type_name.to_string(),
                    external_crate: ext_crate,
                    is_approved: true,
                    is_std: false,
                    what: what_str,
                    in_what_type,
                    location,
                },
            );
        }
        Err(AllowedTypeError::StandardLibraryNotAllowed(_)) => {
            catalog.record_direct_external(
                owner_id,
                DirectExternalUsage {
                    type_name: type_name.to_string(),
                    external_crate: ext_crate,
                    is_approved: false,
                    is_std: true,
                    what: what_str,
                    in_what_type,
                    location,
                },
            );
        }
        Err(AllowedTypeError::NoMatchFound) => {
            catalog.record_direct_external(
                owner_id,
                DirectExternalUsage {
                    type_name: type_name.to_string(),
                    external_crate: ext_crate,
                    is_approved: false,
                    is_std: false,
                    what: what_str,
                    in_what_type,
                    location,
                },
            );
        }
        Err(AllowedTypeError::DuplicateMatches(_)) => {
            catalog.record_direct_external(
                owner_id,
                DirectExternalUsage {
                    type_name: type_name.to_string(),
                    external_crate: ext_crate,
                    is_approved: true,
                    is_std: false,
                    what: what_str,
                    in_what_type,
                    location,
                },
            );
        }
    }
}
