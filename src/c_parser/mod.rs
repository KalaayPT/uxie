//! C header file parsing with full expression evaluation
//!
//! This module provides a complete C preprocessor-like symbol resolution system:
//! - Parse `#define` constants and evaluate complex expressions
//! - Parse C enums with incremental value assignment
//! - Parse `#include` directives and resolve dependencies
//! - Full operator precedence support (arithmetic, bitwise, logical)
//! - Pratt parser implementation for correct C expression evaluation

pub mod defines;
pub mod enums;
pub mod includes;
pub mod source_manager;
pub mod symbol_table;
#[cfg(test)]
mod tests;

pub use defines::{CDefine, parse_and_resolve_defines, parse_defines, parse_value};
pub use enums::{CEnum, CEnumVariant, parse_enum};
pub use source_manager::SourceManager;
pub use symbol_table::{SymbolTable, SymbolTag};
