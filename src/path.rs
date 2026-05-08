/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

use rustdoc_types::{Id, Item, Span};
use std::fmt;

/// Component type for components in a [`Path`].
#[derive(Copy, Clone, Debug)]
pub enum ComponentType {
    AssocConst,
    AssocType,
    Constant,
    Crate,
    Enum,
    EnumVariant,
    Function,
    Impl,
    Module,
    ReExport,
    Static,
    Struct,
    StructField,
    Trait,
    TypeAlias,
    Union,
}

/// Represents one component in a [`Path`].
#[derive(Clone, Debug)]
struct Component {
    typ: ComponentType,
    name: String,
    span: Option<Span>,
    /// Rustdoc `Id` of the underlying item, when known. Components added via
    /// `Path::push_raw` (e.g. for re-exports / impl blocks) have `id = None`.
    id: Option<Id>,
}

impl Component {
    fn new(typ: ComponentType, name: String, span: Option<Span>, id: Option<Id>) -> Self {
        Self {
            typ,
            name,
            span,
            id,
        }
    }
}

/// Represents the full path to an item being visited by [`Visitor`](crate::visitor::Visitor).
///
/// This is equivalent to the type path of that item, which has to be re-assembled since
/// it is lost in the flat structure of the Rustdoc JSON output.
#[derive(Clone, Debug)]
pub struct Path {
    stack: Vec<Component>,
}

impl Path {
    pub fn new(crate_name: &str) -> Self {
        Self {
            stack: vec![Component::new(
                ComponentType::Crate,
                crate_name.into(),
                None,
                None,
            )],
        }
    }

    pub fn push(&mut self, typ: ComponentType, item: &Item) {
        self.stack.push(Component::new(
            typ,
            item.name.as_ref().expect("name").clone(),
            item.span.clone(),
            Some(item.id),
        ));
    }

    pub fn push_raw(&mut self, typ: ComponentType, name: &str, span: Option<&Span>) {
        self.stack
            .push(Component::new(typ, name.into(), span.cloned(), None));
    }

    /// Returns the span (file + beginning and end positions) of the last `Component` in the stack.
    pub fn last_span(&self) -> Option<&Span> {
        self.stack.last().and_then(|c| c.span.as_ref())
    }

    /// Returns the [`ComponentType`] of the last `Component` in the path.
    pub fn last_type(&self) -> Option<ComponentType> {
        self.stack.last().map(|c| c.typ)
    }

    /// Parent path string (all path segments except the last non-empty name).
    pub fn parent_path_string(&self) -> Option<String> {
        let names: Vec<&str> = self
            .stack
            .iter()
            .filter(|c| !c.name.is_empty())
            .map(|c| c.name.as_str())
            .collect();
        if names.len() <= 1 {
            return None;
        }
        Some(names[..names.len() - 1].join("::"))
    }

    /// Nearest owning item for catalog attribution: walks the path stack from
    /// the end and returns the first component with a rustdoc [`Id`] whose
    /// type is a module, type, function, or enum variant.
    pub fn enclosing_catalog_owner_with_id(&self) -> Option<(String, Id, ComponentType)> {
        for i in (0..self.stack.len()).rev() {
            let c = &self.stack[i];
            let id = c.id?;
            let typ = match c.typ {
                ComponentType::Struct
                | ComponentType::Enum
                | ComponentType::Union
                | ComponentType::Trait
                | ComponentType::TypeAlias
                | ComponentType::Function
                | ComponentType::Static
                | ComponentType::Constant
                | ComponentType::Module
                | ComponentType::EnumVariant => c.typ,
                _ => continue,
            };
            let names: Vec<&str> = self.stack[..=i]
                .iter()
                .filter(|c| !c.name.is_empty())
                .map(|c| c.name.as_str())
                .collect();
            return Some((names.join("::"), id, typ));
        }
        None
    }

    /// If the path ends with `Struct(name) -> StructField(...)`, returns the
    /// full type path string up to and including the `Struct` component
    /// together with the rustdoc [`Id`] of that struct.
    ///
    /// Used to detect whether the visitor is currently inspecting the type of
    /// a field of a local struct (as opposed to an enum variant field, union
    /// field, or an item inside an `impl` block), and to identify the struct
    /// uniquely for transitive-exposure analysis.
    pub fn enclosing_local_struct_with_id(&self) -> Option<(String, Id)> {
        let n = self.stack.len();
        if n < 2 {
            return None;
        }
        if !matches!(self.stack[n - 1].typ, ComponentType::StructField) {
            return None;
        }
        if !matches!(self.stack[n - 2].typ, ComponentType::Struct) {
            return None;
        }
        let id = self.stack[n - 2].id?;
        let names: Vec<&str> = self.stack[..n - 1]
            .iter()
            .filter(|c| !c.name.is_empty())
            .map(|c| c.name.as_str())
            .collect();
        Some((names.join("::"), id))
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self
            .stack
            .iter()
            .filter(|component| !component.name.is_empty())
            .map(|component| component.name.as_str())
            .collect();
        write!(f, "{}", names.join("::"))
    }
}
