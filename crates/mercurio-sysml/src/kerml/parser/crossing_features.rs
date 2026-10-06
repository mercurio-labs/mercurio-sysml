//! KerML owned crossing features belong to the following end feature.
use super::*;

impl Parser {
    pub(super) fn parse_crossing_end_feature(
        &mut self,
        modifier_start: usize,
        docs: Vec<String>,
    ) -> Result<Option<GenericUsageDecl>, Diagnostic> {
        let Some(end_index) = (modifier_start..self.index).find(|index| {
            matches!(&self.tokens[*index].kind, TokenKind::Identifier(word) if word == "end")
        }) else { return Ok(None); };
        let Some(feature_index) = (end_index + 1..self.tokens.len())
            .take_while(|index| !matches!(self.tokens[*index].kind,
                TokenKind::Semicolon | TokenKind::LBrace | TokenKind::RBrace | TokenKind::Eof))
            .find(|index| matches!(&self.tokens[*index].kind, TokenKind::Identifier(word) if word == "feature"))
        else { return Ok(None); };
        // KerML FeaturePrefix puts metadata after the optional crossing
        // declaration. A metadata-only prefix does not declare a crossing.
        let crossing_end = (end_index + 1..feature_index)
            .find(|index| matches!(self.tokens[*index].kind, TokenKind::Hash))
            .unwrap_or(feature_index);
        if crossing_end == end_index + 1 {
            return Ok(None);
        }
        let mut header = self.tokens[end_index + 1..crossing_end].to_vec();
        let mut terminator = self.tokens[feature_index].clone();
        terminator.kind = TokenKind::Semicolon;
        header.push(terminator.clone());
        terminator.kind = TokenKind::Eof;
        header.push(terminator.clone());
        let mut parser = Parser::new(header);
        let modifiers = parser.parse_modifiers();
        let mut crossing = parser.parse_unprefixed_feature(Vec::new(), modifiers)?;
        crossing.modifiers.push("owned_crossing_feature".into());
        let mut prefix = self.tokens[modifier_start..=end_index].to_vec();
        prefix.push(terminator);
        let modifiers = Parser::new(prefix).parse_modifiers();
        self.index = feature_index;
        let mut end = self.parse_feature_with_modifiers(docs, modifiers)?;
        end.span = merge_span(&self.tokens[modifier_start].span, &end.span);
        end.body_members
            .insert(0, Declaration::GenericUsage(crossing));
        Ok(Some(end))
    }
}
