//! C header file parsing with strict constant-expression evaluation
//!
//! This module provides a generic C preprocessor-like symbol resolution system:
//! - Parse `#define` constants and evaluate complex expressions
//! - Parse C enums with incremental value assignment
//! - Parse `#include` directives and resolve on-disk dependencies
//! - Pratt-parser precedence for arithmetic, bitwise, shift, comparison, logical, and unary operators
//! - Unsupported constructs (for example ternary, assignment, or unsupported macros) fail explicitly

pub mod constant_cache;
pub mod defines;
pub mod enums;
pub mod includes;
pub mod source_manager;
pub mod symbol_table;
#[cfg(test)]
mod tests;

pub use constant_cache::{ConstantCache, SymbolSnapshot};
pub use defines::{parse_and_resolve_defines, parse_defines, parse_value, CDefine};
pub use enums::{parse_enum, CEnum, CEnumVariant};
pub use source_manager::SourceManager;
pub use symbol_table::{canonicalize_constant_name, ConstantFamily, SymbolTable, SymbolTag};
