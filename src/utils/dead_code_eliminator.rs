use crate::utils::Logger;

/// Dead code eliminator for conditional exports based on environment variables
pub struct DeadCodeEliminator;

impl DeadCodeEliminator {
    pub fn new() -> Self {
        Self
    }

    /// Eliminate dead code branches based on constant conditions
    /// This runs after environment variable replacement
    pub fn eliminate(&self, code: &str) -> String {
        Logger::debug("🗑️  Running dead code elimination");

        let mut result = code.to_string();
        let mut eliminated_count = 0;

        // Pattern 1: if (false) { ... }
        result = self.eliminate_false_blocks(&result, &mut eliminated_count);

        // Pattern 2: if (true) { ... } else { ... } -> keep only if block
        result = self.eliminate_true_blocks(&result, &mut eliminated_count);

        // Pattern 3: condition ? expr1 : expr2 with known condition
        result = self.eliminate_ternary(&result, &mut eliminated_count);

        // Pattern 4: false && expr -> remove
        result = self.eliminate_false_and(&result, &mut eliminated_count);

        // Pattern 5: true || expr -> true
        result = self.eliminate_true_or(&result, &mut eliminated_count);

        if eliminated_count > 0 {
            Logger::debug(&format!(
                "🗑️  Eliminated {} dead code branches",
                eliminated_count
            ));
        }

        result
    }

    /// Eliminate if (false) { ... } blocks
    fn eliminate_false_blocks(&self, code: &str, count: &mut usize) -> String {
        let mut result = code.to_string();

        // Simple pattern: if(false){...}
        // Note: After minification, spaces are removed
        loop {
            if let Some(start) = result.find("if(false)") {
                if let Some(block_start) = result[start..].find('{') {
                    let block_start_abs = start + block_start;
                    if let Some(block_end) = self.find_matching_brace(&result, block_start_abs) {
                        // Check if there's an else clause
                        let after_block = &result[block_end + 1..];
                        let trimmed = after_block.trim_start();

                        if trimmed.starts_with("else") {
                            // if (false) { ... } else { ... } -> keep else block
                            let else_start = block_end + 1 + after_block.len() - trimmed.len();
                            let else_keyword_end = else_start + 4; // "else".len()

                            let after_else = result[else_keyword_end..].trim_start();
                            if after_else.starts_with('{') {
                                // else { ... }
                                let else_block_start = else_keyword_end
                                    + (result[else_keyword_end..].len() - after_else.len());
                                if let Some(else_block_end) =
                                    self.find_matching_brace(&result, else_block_start)
                                {
                                    // Extract else block content and replace entire if-else
                                    let else_content =
                                        result[else_block_start + 1..else_block_end].to_string();
                                    result.replace_range(start..else_block_end + 1, &else_content);
                                    *count += 1;
                                    continue;
                                }
                            }
                        } else {
                            // No else clause, just remove the if block
                            result.replace_range(start..block_end + 1, "");
                            *count += 1;
                            continue;
                        }
                    }
                }
            }
            break;
        }

        result
    }

    /// Eliminate if (true) { ... } else { ... } -> keep only if block
    fn eliminate_true_blocks(&self, code: &str, count: &mut usize) -> String {
        let mut result = code.to_string();

        loop {
            if let Some(start) = result.find("if(true)") {
                if let Some(block_start) = result[start..].find('{') {
                    let block_start_abs = start + block_start;
                    if let Some(block_end) = self.find_matching_brace(&result, block_start_abs) {
                        // Extract if block content
                        let if_content = &result[block_start_abs + 1..block_end].to_string();

                        // Check if there's an else clause to remove
                        let after_block = &result[block_end + 1..];
                        let trimmed = after_block.trim_start();

                        if trimmed.starts_with("else") {
                            // Find end of else clause
                            let else_start = block_end + 1 + after_block.len() - trimmed.len();
                            let else_keyword_end = else_start + 4;

                            let after_else = result[else_keyword_end..].trim_start();
                            if after_else.starts_with('{') {
                                let else_block_start = else_keyword_end
                                    + (result[else_keyword_end..].len() - after_else.len());
                                if let Some(else_block_end) =
                                    self.find_matching_brace(&result, else_block_start)
                                {
                                    // Replace if-else with just the if content
                                    result.replace_range(start..else_block_end + 1, if_content);
                                    *count += 1;
                                    continue;
                                }
                            }
                        } else {
                            // No else, just unwrap the if block
                            result.replace_range(start..block_end + 1, if_content);
                            *count += 1;
                            continue;
                        }
                    }
                }
            }
            break;
        }

        result
    }

    /// Eliminate ternary expressions with known conditions
    fn eliminate_ternary(&self, code: &str, count: &mut usize) -> String {
        let mut result = code.to_string();

        // true ? a : b -> a
        while let Some(pos) = result.find("true?") {
            if let Some(colon) = result[pos..].find(':') {
                let colon_abs = pos + colon;
                // Find the end of the false branch (simplified)
                // This is a basic implementation, full parser would be better
                if let Some(end) = self.find_expression_end(&result, colon_abs + 1) {
                    let true_branch_start = pos + 5; // "true?".len()
                    let true_branch = &result[true_branch_start..colon_abs].trim().to_string();
                    result.replace_range(pos..end, true_branch);
                    *count += 1;
                    continue;
                }
            }
            break;
        }

        // false ? a : b -> b
        while let Some(pos) = result.find("false?") {
            if let Some(colon) = result[pos..].find(':') {
                let colon_abs = pos + colon;
                if let Some(end) = self.find_expression_end(&result, colon_abs + 1) {
                    let false_branch_start = colon_abs + 1;
                    let false_branch = &result[false_branch_start..end].trim().to_string();
                    result.replace_range(pos..end, false_branch);
                    *count += 1;
                    continue;
                }
            }
            break;
        }

        result
    }

    /// Eliminate false && expr -> remove
    fn eliminate_false_and(&self, code: &str, count: &mut usize) -> String {
        let mut result = code.to_string();

        // false && anything -> false
        while let Some(pos) = result.find("false&&") {
            if let Some(end) = self.find_expression_end(&result, pos + 7) {
                result.replace_range(pos..end, "false");
                *count += 1;
                continue;
            }
            break;
        }

        result
    }

    /// Eliminate true || expr -> true
    fn eliminate_true_or(&self, code: &str, count: &mut usize) -> String {
        let mut result = code.to_string();

        // true || anything -> true
        while let Some(pos) = result.find("true||") {
            if let Some(end) = self.find_expression_end(&result, pos + 6) {
                result.replace_range(pos..end, "true");
                *count += 1;
                continue;
            }
            break;
        }

        result
    }

    /// Find the matching closing brace for an opening brace
    fn find_matching_brace(&self, code: &str, open_pos: usize) -> Option<usize> {
        if code.as_bytes().get(open_pos) != Some(&b'{') {
            return None;
        }

        let mut depth = 1;
        let bytes = code.as_bytes();

        for (i, &byte) in bytes.iter().enumerate().skip(open_pos + 1) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Find the end of an expression (simplified heuristic)
    fn find_expression_end(&self, code: &str, start: usize) -> Option<usize> {
        let bytes = code.as_bytes();
        let mut paren_depth = 0;

        for (i, &byte) in bytes.iter().enumerate().skip(start) {
            match byte {
                b'(' => paren_depth += 1,
                b')' => {
                    if paren_depth == 0 {
                        return Some(i);
                    }
                    paren_depth -= 1;
                }
                b';' | b',' | b'}' if paren_depth == 0 => return Some(i),
                _ => {}
            }
        }

        Some(bytes.len())
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eliminate_false_if() {
        let eliminator = DeadCodeEliminator::new();
        let code = "console.log('before');if(false){console.log('never');}console.log('after');";
        let result = eliminator.eliminate(code);
        assert!(!result.contains("console.log('never')"));
        assert!(result.contains("console.log('before')"));
        assert!(result.contains("console.log('after')"));
    }

    #[test]
    fn test_eliminate_false_if_with_else() {
        let eliminator = DeadCodeEliminator::new();
        let code = "if(false){console.log('never');}else{console.log('always');}";
        let result = eliminator.eliminate(code);
        assert!(!result.contains("never"));
        assert!(result.contains("always"));
    }

    #[test]
    fn test_eliminate_true_if_with_else() {
        let eliminator = DeadCodeEliminator::new();
        let code = "if(true){console.log('always');}else{console.log('never');}";
        let result = eliminator.eliminate(code);
        assert!(result.contains("always"));
        assert!(!result.contains("never"));
    }

    #[test]
    fn test_eliminate_true_ternary() {
        let eliminator = DeadCodeEliminator::new();
        let code = "const x=true?'yes':'no';";
        let result = eliminator.eliminate(code);
        assert!(result.contains("'yes'"));
        assert!(!result.contains("'no'"));
    }

    #[test]
    fn test_eliminate_false_ternary() {
        let eliminator = DeadCodeEliminator::new();
        let code = "const x=false?'yes':'no';";
        let result = eliminator.eliminate(code);
        assert!(!result.contains("'yes'"));
        assert!(result.contains("'no'"));
    }

    #[test]
    fn test_eliminate_false_and() {
        let eliminator = DeadCodeEliminator::new();
        let code = "if(false&&expensive()){doSomething();}";
        let result = eliminator.eliminate(code);
        assert!(!result.contains("expensive"));
    }
}
