//! Accessors for bare expansion lines.
//!
//! A top-level line that holds only a function call or variable expansion,
//! such as `$(info ...)` or `$(eval ...)`, is kept as an [`Expansion`] item.
//! GNU Make expands the line and parses the result, which a lossless parser
//! cannot do, so these accessors expose the references and leave the caller
//! to decide what the line may define.

use crate::lossless::{Expansion, VariableReference};
use crate::SyntaxKind::*;
use rowan::ast::AstNode;

impl Expansion {
    /// Returns the references written directly on the line, in order.
    ///
    /// References nested inside another reference's arguments are not
    /// included; reach them through [`VariableReference::syntax`].
    ///
    /// # Example
    /// ```
    /// use makefile_lossless::{Makefile, MakefileItem};
    /// let makefile: Makefile = "$(info hello) $(FOO)\n".parse().unwrap();
    /// let Some(MakefileItem::Expansion(line)) = makefile.items().next() else {
    ///     panic!("expected an expansion line");
    /// };
    /// let names: Vec<_> = line.references().filter_map(|r| r.name()).collect();
    /// assert_eq!(names, ["info", "FOO"]);
    /// ```
    pub fn references(&self) -> impl Iterator<Item = VariableReference> + '_ {
        self.syntax().children().filter_map(VariableReference::cast)
    }

    /// Returns true if the line holds nothing but references, whitespace,
    /// line continuations and a trailing comment.
    ///
    /// A line with literal text beside its references, such as
    /// `$(PREFIX)-extra`, returns false.
    ///
    /// # Example
    /// ```
    /// use makefile_lossless::{Makefile, MakefileItem};
    /// let makefile: Makefile = "$(info hello) # greet\n".parse().unwrap();
    /// let Some(MakefileItem::Expansion(line)) = makefile.items().next() else {
    ///     panic!("expected an expansion line");
    /// };
    /// assert!(line.has_only_references());
    /// ```
    pub fn has_only_references(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .all(|token| {
                matches!(
                    token.kind(),
                    WHITESPACE | NEWLINE | COMMENT | BACKSLASH | INDENT
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lossless::parse;

    fn expansion_of(input: &str) -> Expansion {
        let parsed = parse(input, None);
        assert!(
            parsed.errors.is_empty(),
            "unexpected errors: {:?}",
            parsed.errors
        );
        parsed
            .root()
            .syntax()
            .descendants()
            .find_map(Expansion::cast)
            .expect("no EXPANSION node")
    }

    #[test]
    fn test_references_of_a_single_call() {
        let line = expansion_of("$(error KANI_VERSION must be MAJOR.MINOR.PATCH)\n");
        let references: Vec<_> = line.references().collect();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].name(), Some("error".to_string()));
        assert!(references[0].is_function_call());
    }

    #[test]
    fn test_references_skip_nested_arguments() {
        let line = expansion_of("$(eval $(call define-rule,$(TARGET)))\n");
        let names: Vec<_> = line.references().filter_map(|r| r.name()).collect();
        assert_eq!(names, ["eval"]);
    }

    #[test]
    fn test_a_bare_variable_is_not_a_function_call() {
        let line = expansion_of("$(RULES)\n");
        let references: Vec<_> = line.references().collect();
        assert_eq!(references.len(), 1);
        assert!(!references[0].is_function_call());
    }

    #[test]
    fn test_only_references_with_a_comment_and_continuation() {
        let line = expansion_of("$(info a) \\\n  $(info b) # both\n");
        assert!(line.has_only_references());
        assert_eq!(line.references().count(), 2);
    }

    #[test]
    fn test_literal_text_beside_a_reference() {
        let line = expansion_of("$(PREFIX)-extra\n");
        assert!(!line.has_only_references());
    }
}
