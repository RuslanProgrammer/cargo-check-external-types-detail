/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::bug;
use anyhow::{Context, Result};
use pest::Position;
use rustdoc_types::Span;
use serde_json::{json, Value as JsonValue};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use wildmatch::WildMatch;

/// Where the error occurred relative to the [`Path`](crate::path::Path).
///
/// For example, if the path is a path to a function, then this could point to something
/// specific about that function, such as a specific function argument that is in error.
///
/// There is overlap in this enum with [`ComponentType`](crate::path::ComponentType) since
/// some paths are specific enough to locate the external type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorLocation {
    AssocType,
    ArgumentNamed(String),
    ClosureInput,
    ClosureOutput,
    ConstGeneric,
    Constant,
    DynTrait,
    EnumTupleEntry,
    GenericArg,
    GenericDefaultBinding,
    ImplementedTrait,
    QualifiedSelfType,
    QualifiedSelfTypeAsTrait,
    ReExport,
    ReturnValue,
    Static,
    StructField,
    TraitBound,
    TypeAlias,
    WhereBound,
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AssocType => "associated type",
            Self::ArgumentNamed(name) => return write!(f, "argument named `{name}` of"),
            Self::ClosureInput => "closure input of",
            Self::ClosureOutput => "closure output of",
            Self::ConstGeneric => "const generic of",
            Self::Constant => "constant",
            Self::DynTrait => "dyn trait of",
            Self::EnumTupleEntry => "enum tuple entry of",
            Self::GenericArg => "generic arg of",
            Self::GenericDefaultBinding => "generic default binding of",
            Self::ImplementedTrait => "implemented trait of",
            Self::QualifiedSelfType => "qualified self type",
            Self::QualifiedSelfTypeAsTrait => "qualified type `as` trait",
            Self::ReExport => "re-export named",
            Self::ReturnValue => "return value of",
            Self::Static => "static value",
            Self::StructField => "struct field of",
            Self::TraitBound => "trait bound of",
            Self::TypeAlias => "type alias of",
            Self::WhereBound => "where bound of",
        };
        write!(f, "{s}")
    }
}

/// First path segment of a fully qualified type path (the external crate name).
fn external_crate_of_type_name(type_name: &str) -> Option<&str> {
    type_name.split("::").next()
}

fn escape_markdown_table_cell(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\n', '\r'], " ")
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

fn fmt_local_struct_chain_headline(
    chain: &[String],
    type_name: &str,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if chain.is_empty() {
        return Ok(());
    }
    write!(f, "Local struct `{}`", chain[0])?;
    for (i, link) in chain.iter().skip(1).enumerate() {
        if i == 0 {
            write!(f, " contains struct `{link}`")?;
        } else {
            write!(f, " that contains struct `{link}`")?;
        }
    }
    if chain.len() == 1 {
        write!(f, " exposes unapproved external type `{type_name}`")
    } else {
        write!(f, " that exposes unapproved external type `{type_name}`")
    }
}

#[derive(Default)]
pub struct ValidationErrors {
    errors: BTreeSet<ValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn error_count(&self) -> usize {
        self.errors
            .iter()
            .map(ValidationError::level)
            .filter(|&l| l == ErrorLevel::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.errors
            .iter()
            .map(ValidationError::level)
            .filter(|&l| l == ErrorLevel::Warning)
            .count()
    }

    pub fn add(&mut self, error: ValidationError) {
        self.errors.insert(error);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Renders the entire diagnostic set as a single JSON object suitable for
    /// `--output-format json`. The object has stable summary fields plus a
    /// `diagnostics` array.
    ///
    /// Note: [`ValidationError::markdown_table_row`] only surfaces external-type
    /// diagnostics, while JSON includes every diagnostic (warnings, unused
    /// patterns, etc.).
    pub fn to_json_value(&self) -> JsonValue {
        let diagnostics: Vec<JsonValue> = self.iter().map(ValidationError::to_json_value).collect();
        json!({
            "summary": {
                "errors_count": self.error_count(),
                "warnings_count": self.warning_count(),
                "diagnostics_count": diagnostics.len(),
            },
            "diagnostics": diagnostics,
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ErrorLevel {
    Error,
    Warning,
}

impl ErrorLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// Error type for validation errors that get displayed to the user on the CLI.
#[derive(Debug)]
pub enum ValidationError {
    UnapprovedExternalTypeRef {
        type_name: String,
        what: ErrorLocation,
        in_what_type: String,
        location: Option<Span>,
        sort_key: String,
    },
    FieldsStripped {
        type_name: String,
    },
    HiddenModule {
        type_name: String,
        what: ErrorLocation,
        in_what_type: String,
        location: Option<Span>,
        hidden_module: Option<String>,
    },
    HiddenItem {
        what: ErrorLocation,
        in_what_type: String,
        location: Option<Span>,
        sort_key: String,
    },
    UnusedApprovalPattern {
        type_name: String,
    },
    DuplicateApproved {
        type_name: String,
        what: ErrorLocation,
        in_what_type: String,
        location: Option<Span>,
        duplicate: Vec<String>,
        sort_key: String,
    },
    LocalStructExposesExternalType {
        /// Chain of local-struct paths from the public-API entry point to the
        /// struct that directly exposes the external type. Length 1 means the
        /// entry struct itself directly exposes the type. Length > 1 means the
        /// exposure is transitive through intermediate local structs.
        chain: Vec<String>,
        type_name: String,
        /// Where the exposure appears in the public API (e.g. full path to a struct field).
        what: ErrorLocation,
        in_what_type: String,
        location: Option<Span>,
        sort_key: String,
    },
}

/// Labels the two external-exposure rows in [`ValidationError::markdown_table_row`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownExposureKind {
    UnapprovedExternal,
    LocalStructChain,
}

impl MarkdownExposureKind {
    fn as_table_value(self) -> &'static str {
        match self {
            Self::UnapprovedExternal => "unapproved_external",
            Self::LocalStructChain => "local_struct_chain",
        }
    }
}

/// One row for `--output-format markdown-table` (cells are already escaped for `|`).
#[derive(Debug, Clone)]
pub struct MarkdownTableRow {
    pub external_crate: String,
    pub external_type: String,
    pub exposure_kind: MarkdownExposureKind,
    /// Full API path of the exposure (markdown column "API path").
    pub path_in_api: String,
    pub local_struct_chain: String,
    pub role: String,
    pub source: String,
}

impl MarkdownTableRow {
    pub fn format_line(&self) -> String {
        format!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            self.external_crate,
            self.external_type,
            self.exposure_kind.as_table_value(),
            self.path_in_api,
            self.local_struct_chain,
            self.role,
            self.source,
        )
    }
}

impl ValidationError {
    pub fn unapproved_external_type_ref(
        type_name: impl Into<String>,
        what: &ErrorLocation,
        in_what_type: impl Into<String>,
        location: Option<&Span>,
    ) -> Self {
        let type_name = type_name.into();
        let in_what_type = in_what_type.into();
        let sort_key = format!(
            "{}:{type_name}:{what}:{in_what_type}",
            location_sort_key(location)
        );
        if location.is_none() {
            bug!("An error is missing a span and will be printed without context, file name, and line number.");
        }
        Self::UnapprovedExternalTypeRef {
            type_name,
            what: what.clone(),
            in_what_type,
            location: location.cloned(),
            sort_key,
        }
    }

    pub fn level(&self) -> ErrorLevel {
        match self {
            Self::UnapprovedExternalTypeRef { .. }
            | Self::LocalStructExposesExternalType { .. } => ErrorLevel::Error,
            Self::HiddenModule { .. }
            | Self::HiddenItem { .. }
            | Self::FieldsStripped { .. }
            | Self::UnusedApprovalPattern { .. }
            | Self::DuplicateApproved { .. } => ErrorLevel::Warning,
        }
    }

    pub fn fields_stripped(path: &crate::path::Path) -> Self {
        Self::FieldsStripped {
            type_name: path.to_string(),
        }
    }

    pub fn hidden_module(
        type_name: impl Into<String>,
        what: &ErrorLocation,
        in_what_type: impl Into<String>,
        location: Option<&Span>,
        hidden_module: Option<String>,
    ) -> Self {
        if location.is_none() {
            bug!("A warning is missing a span and will be printed without context, file name, and line number.");
        }
        Self::HiddenModule {
            type_name: type_name.into(),
            what: what.clone(),
            in_what_type: in_what_type.into(),
            location: location.cloned(),
            hidden_module,
        }
    }

    pub fn hidden_item(
        what: &ErrorLocation,
        in_what_type: impl Into<String>,
        location: Option<&Span>,
    ) -> Self {
        if location.is_none() {
            bug!("A warning is missing a span and will be printed without context, file name, and line number.");
        }
        Self::HiddenItem {
            what: what.clone(),
            in_what_type: in_what_type.into(),
            location: location.cloned(),
            sort_key: location_sort_key(location),
        }
    }

    pub fn unused_approval_pattern(type_name: impl Into<String>) -> Self {
        Self::UnusedApprovalPattern {
            type_name: type_name.into(),
        }
    }

    pub fn duplicate_approved(
        type_name: impl Into<String>,
        what: &ErrorLocation,
        in_what_type: impl Into<String>,
        location: Option<&Span>,
        duplicate: Vec<&WildMatch>,
    ) -> Self {
        if location.is_none() {
            bug!("A warning is missing a span and will be printed without context, file name, and line number.");
        }
        let type_name = type_name.into();
        let in_what_type = in_what_type.into();
        let duplicate = duplicate
            .iter()
            .map(|pattern| pattern.to_string())
            .collect();
        let sort_key = format!(
            "{}:{type_name}:{what}:{in_what_type}",
            location_sort_key(location)
        );
        Self::DuplicateApproved {
            type_name,
            what: what.clone(),
            in_what_type,
            location: location.cloned(),
            duplicate,
            sort_key,
        }
    }

    pub fn local_struct_exposes_external_type(
        chain: Vec<String>,
        type_name: impl Into<String>,
        original_what: &ErrorLocation,
        original_in_what_type: impl Into<String>,
        location: Option<&Span>,
    ) -> Self {
        let type_name = type_name.into();
        let in_what_type = original_in_what_type.into();
        // Sort directly AFTER the corresponding `UnapprovedExternalTypeRef` for the
        // same field by using its sort key as a prefix and appending `:exposes`.
        // Because `"X" < "X:exposes"` lexicographically, this yields the alternating
        // ordering shown in the expected output.
        let sort_key = format!(
            "{}:{type_name}:{original_what}:{}:exposes",
            location_sort_key(location),
            in_what_type
        );
        if location.is_none() {
            bug!("An error is missing a span and will be printed without context, file name, and line number.");
        }
        if chain.is_empty() {
            bug!("local_struct_exposes_external_type called with empty chain.");
        }
        Self::LocalStructExposesExternalType {
            chain,
            type_name,
            what: original_what.clone(),
            in_what_type,
            location: location.cloned(),
            sort_key,
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            Self::UnapprovedExternalTypeRef { type_name, .. }
            | Self::HiddenModule { type_name, .. }
            | Self::FieldsStripped { type_name }
            | Self::UnusedApprovalPattern { type_name }
            | Self::DuplicateApproved { type_name, .. }
            | Self::LocalStructExposesExternalType { type_name, .. } => type_name,
            Self::HiddenItem { .. } => "N/A",
        }
    }

    pub fn location(&self) -> Option<&Span> {
        match self {
            Self::UnapprovedExternalTypeRef { location, .. }
            | Self::HiddenModule { location, .. }
            | Self::HiddenItem { location, .. }
            | Self::DuplicateApproved { location, .. }
            | Self::LocalStructExposesExternalType { location, .. } => location.as_ref(),
            Self::FieldsStripped { .. } | Self::UnusedApprovalPattern { .. } => None,
        }
    }

    fn sort_key(&self) -> &str {
        match self {
            Self::UnapprovedExternalTypeRef { sort_key, .. }
            | Self::DuplicateApproved { sort_key, .. }
            | Self::LocalStructExposesExternalType { sort_key, .. } => sort_key.as_ref(),
            Self::FieldsStripped { type_name }
            | Self::HiddenModule { type_name, .. }
            | Self::UnusedApprovalPattern { type_name } => type_name.as_ref(),
            Self::HiddenItem { sort_key, .. } => sort_key.as_ref(),
        }
    }

    pub fn fmt_headline(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnapprovedExternalTypeRef { type_name, .. } => {
                write!(
                    f,
                    "Unapproved external type `{type_name}` referenced in public API"
                )
            }
            Self::HiddenModule {
                type_name,
                hidden_module,
                ..
            } => {
                let hidden_module = hidden_module.as_deref().unwrap_or("???");
                write!(
                     f,
                     "Module path for reexported type `{type_name}` contains a `#[doc(hidden)]` module \"{hidden_module}\". Types declared in this module cannot be checked for external types"
                 )
            }
            Self::HiddenItem {
                what, in_what_type, ..
            } => {
                write!(
                     f,
                     "{what} {in_what_type} references a hidden item. Items marked `#[doc(hidden)]` cannot be checked for external types"
                 )
            }
            Self::FieldsStripped { type_name } => {
                write!(
                     f,
                     "Fields on `{type_name}` marked `#[doc(hidden)]` cannot be checked for external types"
                 )
            }
            Self::UnusedApprovalPattern { type_name } => {
                write!(
                    f,
                    "Approved external type `{type_name}` wasn't referenced in public API"
                )
            }
            Self::DuplicateApproved {
                type_name,
                duplicate,
                ..
            } => {
                write!(
                    f,
                    "External type `{type_name}` is allowed multiple times:\n Allowed patterns:{}",
                    duplicate
                        .iter()
                        .map(|glob| format!("\n    - {glob}"))
                        .fold(String::new(), |acc, f| acc + &f)
                )
            }
            Self::LocalStructExposesExternalType {
                chain,
                type_name,
                ..
            } => fmt_local_struct_chain_headline(chain, type_name, f),
        }
    }

    pub fn subtext(&self) -> Cow<'static, str> {
        match self {
            Self::UnapprovedExternalTypeRef {
                what, in_what_type, ..
            } => format!("in {what} `{in_what_type}`").into(),
            Self::FieldsStripped { .. } | Self::UnusedApprovalPattern { .. } => "".into(),
            Self::HiddenModule {
                what, in_what_type, ..
            }
            | Self::HiddenItem {
                what, in_what_type, ..
            }
            | Self::DuplicateApproved {
                what, in_what_type, ..
            } => format!("in {what} `{in_what_type}`").into(),
            Self::LocalStructExposesExternalType { chain, .. } => {
                format!("in local struct `{}`", chain[0]).into()
            }
        }
    }

    /// Stable machine-readable identifier for each diagnostic variant. Used
    /// in the `markdown-table` and `json` output formats so consumers can
    /// classify diagnostics without parsing the human-readable headline.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::UnapprovedExternalTypeRef { .. } => "unapproved_external_type_ref",
            Self::FieldsStripped { .. } => "fields_stripped",
            Self::HiddenModule { .. } => "hidden_module",
            Self::HiddenItem { .. } => "hidden_item",
            Self::UnusedApprovalPattern { .. } => "unused_approval_pattern",
            Self::DuplicateApproved { .. } => "duplicate_approved",
            Self::LocalStructExposesExternalType { .. } => "local_struct_exposes_external_type",
        }
    }

    /// Renders this diagnostic as a structured JSON object. Used by
    /// `--output-format json` so downstream tools can consume diagnostics
    /// programmatically.
    pub fn to_json_value(&self) -> JsonValue {
        let headline = format!("{self}");
        let level = self.level().as_str();
        let kind = self.kind_str();

        let mut base = json!({
            "level": level,
            "kind": kind,
            "message": headline,
            "location": json_location_object(self.location()),
        });
        let obj = base.as_object_mut().expect("object");

        match self {
            Self::UnapprovedExternalTypeRef {
                type_name,
                what,
                in_what_type,
                ..
            } => {
                obj.insert("type_name".into(), json!(type_name));
                obj.insert(
                    "external_crate".into(),
                    json!(external_crate_of_type_name(type_name)),
                );
                obj.insert("what".into(), json!(what.to_string()));
                obj.insert("in_what_type".into(), json!(in_what_type));
            }
            Self::LocalStructExposesExternalType {
                chain,
                type_name,
                what,
                in_what_type,
                ..
            } => {
                obj.insert("type_name".into(), json!(type_name));
                obj.insert(
                    "external_crate".into(),
                    json!(external_crate_of_type_name(type_name)),
                );
                obj.insert("chain".into(), json!(chain));
                obj.insert("what".into(), json!(what.to_string()));
                obj.insert("in_what_type".into(), json!(in_what_type));
            }
            Self::HiddenModule {
                type_name,
                what,
                in_what_type,
                hidden_module,
                ..
            } => {
                obj.insert("type_name".into(), json!(type_name));
                obj.insert("what".into(), json!(what.to_string()));
                obj.insert("in_what_type".into(), json!(in_what_type));
                obj.insert("hidden_module".into(), json!(hidden_module));
            }
            Self::HiddenItem {
                what, in_what_type, ..
            } => {
                obj.insert("what".into(), json!(what.to_string()));
                obj.insert("in_what_type".into(), json!(in_what_type));
            }
            Self::FieldsStripped { type_name } => {
                obj.insert("type_name".into(), json!(type_name));
            }
            Self::UnusedApprovalPattern { type_name } => {
                obj.insert("type_name".into(), json!(type_name));
            }
            Self::DuplicateApproved {
                type_name,
                what,
                in_what_type,
                duplicate,
                ..
            } => {
                obj.insert("type_name".into(), json!(type_name));
                obj.insert("what".into(), json!(what.to_string()));
                obj.insert("in_what_type".into(), json!(in_what_type));
                obj.insert("duplicate_patterns".into(), json!(duplicate));
            }
        }

        base
    }

    /// One row for `--output-format markdown-table`. Returns `None` for
    /// diagnostics that are not about an unapproved external type surface
    /// (warnings, duplicate-allow patterns, etc.).
    pub fn markdown_table_row(&self) -> Option<MarkdownTableRow> {
        let source_cell = |loc: &Span| {
            format!(
                "{}:{}:{}",
                loc.filename.to_string_lossy(),
                loc.begin.0,
                loc.begin.1
            )
        };
        match self {
            Self::UnapprovedExternalTypeRef {
                type_name,
                what,
                in_what_type,
                location,
                ..
            } => {
                let loc = location.as_ref()?;
                let crate_name = external_crate_of_type_name(type_name)
                    .unwrap_or(type_name.as_str())
                    .to_string();
                Some(MarkdownTableRow {
                    external_crate: escape_markdown_table_cell(&crate_name),
                    external_type: escape_markdown_table_cell(type_name),
                    exposure_kind: MarkdownExposureKind::UnapprovedExternal,
                    path_in_api: escape_markdown_table_cell(in_what_type),
                    local_struct_chain: String::new(),
                    role: escape_markdown_table_cell(&what.to_string()),
                    source: source_cell(loc),
                })
            }
            Self::LocalStructExposesExternalType {
                chain,
                type_name,
                what,
                in_what_type,
                location,
                ..
            } => {
                let loc = location.as_ref()?;
                let crate_name = external_crate_of_type_name(type_name)
                    .unwrap_or(type_name.as_str())
                    .to_string();
                let chain_str = chain.join(" → ");
                Some(MarkdownTableRow {
                    external_crate: escape_markdown_table_cell(&crate_name),
                    external_type: escape_markdown_table_cell(type_name),
                    exposure_kind: MarkdownExposureKind::LocalStructChain,
                    path_in_api: escape_markdown_table_cell(in_what_type),
                    local_struct_chain: escape_markdown_table_cell(&chain_str),
                    role: escape_markdown_table_cell(&what.to_string()),
                    source: source_cell(loc),
                })
            }
            _ => None,
        }
    }
}

fn location_sort_key(location: Option<&Span>) -> String {
    if let Some(location) = location {
        format!(
            "{}:{:07}:{:07}",
            location.filename.to_string_lossy(),
            location.begin.0,
            location.begin.1
        )
    } else {
        "none".into()
    }
}

impl Ord for ValidationError {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(other.sort_key())
    }
}

impl PartialOrd for ValidationError {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for ValidationError {}

impl PartialEq for ValidationError {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_headline(f)
    }
}

/// Pretty printer for error context.
///
/// This makes validation errors look similar to the compiler errors from rustc.
pub struct ErrorPrinter {
    workspace_root: PathBuf,
    file_cache: HashMap<PathBuf, String>,
}

impl ErrorPrinter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            file_cache: HashMap::new(),
        }
    }

    fn get_file_contents(&mut self, path: &Path) -> Result<&str> {
        if !self.file_cache.contains_key(path) {
            let full_file_name = self.workspace_root.join(path).canonicalize()?;
            let contents = std::fs::read_to_string(&full_file_name)
                .context("failed to load source file for error context")
                .context(full_file_name.to_string_lossy().to_string())?;
            self.file_cache.insert(path.to_path_buf(), contents);
        }
        Ok(self.file_cache.get(path).unwrap())
    }

    fn print_error_level(level: ErrorLevel) {
        use owo_colors::{OwoColorize, Stream};
        match level {
            ErrorLevel::Error => {
                print!(
                    "{}",
                    "error: "
                        .if_supports_color(Stream::Stdout, |text| text.red())
                        .if_supports_color(Stream::Stdout, |text| text.bold())
                );
            }
            ErrorLevel::Warning => {
                print!(
                    "{}",
                    "warning: "
                        .if_supports_color(Stream::Stdout, |text| text.yellow())
                        .if_supports_color(Stream::Stdout, |text| text.bold())
                );
            }
        }
    }

    /// Outputs a human readable error with file location context
    ///
    /// # Example output
    ///
    /// ```text
    /// error: Unapproved external type `external_lib::SomeStruct` referenced in public API
    ///    --> test-crate/src/lib.rs:38:1
    ///    |
    /// 38 | pub fn external_in_fn_input(_one: &SomeStruct, _two: impl SimpleTrait) {}
    ///    | ^-----------------------------------------------------------------------^
    ///    |
    ///    = in argument named `_one` of `test_crate::external_in_fn_input`
    /// ```
    pub fn pretty_print_error_context(&mut self, location: &Span, subtext: &str) {
        match self.get_file_contents(&location.filename) {
            Ok(file_contents) => {
                let begin = Self::position_from_line_col(file_contents, location.begin);
                let end = Self::position_from_line_col(file_contents, location.end);

                // HACK: Using Pest to do the pretty error context formatting for lack of
                // knowledge of a smaller library tailored to this use-case
                let variant = pest::error::ErrorVariant::<()>::CustomError {
                    message: subtext.into(),
                };
                let err_context = match (begin, end) {
                    (Some(b), Some(e)) => {
                        Some(pest::error::Error::new_from_span(variant, b.span(&e)))
                    }
                    (Some(b), None) => Some(pest::error::Error::new_from_pos(variant, b)),
                    _ => None,
                };
                if let Some(err_context) = err_context {
                    println!(
                        "{}\n",
                        err_context.with_path(&location.filename.to_string_lossy())
                    );
                }
            }
            Err(err) => {
                Self::print_error_level(ErrorLevel::Error);
                println!("{subtext}");
                println!(
                    "  --> {}:{}:{}",
                    location.filename.to_string_lossy(),
                    location.begin.0,
                    location.begin.1,
                );
                println!("   | Failed to load {:?}", location.filename);
                println!("   | relative to {:?}", self.workspace_root);
                println!("   | to provide error message context.");
                println!("   | Cause: {err:?}");
            }
        }
    }

    fn position_from_line_col(contents: &str, (line, col): (usize, usize)) -> Option<Position<'_>> {
        let (mut cl, mut cc) = (1, 1);
        let content_bytes = contents.as_bytes();
        for (index, &byte) in content_bytes.iter().enumerate() {
            if cl == line && cc == col {
                return Position::new(contents, index);
            }

            cc += 1;
            if byte == b'\n' {
                cl += 1;
                cc = 1;
            }
        }
        None
    }

    pub fn pretty_print_errors(&mut self, errors: &ValidationErrors) {
        for error in errors.iter() {
            Self::print_error_level(error.level());
            println!("{error}");
            if let Some(location) = error.location() {
                self.pretty_print_error_context(location, error.subtext().as_ref())
            }
        }
        if !errors.is_empty() {
            use owo_colors::{OwoColorize, Stream};
            let (error_count, warning_count) = (errors.error_count(), errors.warning_count());
            println!(
                "{error_count} {errors}, {warning_count} {warnings} emitted",
                errors = "errors".if_supports_color(Stream::Stdout, |text| text.red()),
                warnings = "warnings".if_supports_color(Stream::Stdout, |text| text.yellow())
            );
        }
    }
}
