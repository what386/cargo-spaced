use anyhow::{Result, anyhow};
use ra_ap_syntax::{
    AstNode, Edition, SyntaxNode, TextRange,
    ast::{self, HasAttrs},
};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    ItemList,
    StatementList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    BlockBodiedItem,
    LetStatement,
    BlockExpressionStatement,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfo {
    pub range: Range<usize>,
    pub leading_range: Range<usize>,
    pub trailing_range: Range<usize>,
    pub kind: NodeKind,
    pub multiline: bool,
    pub skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub parent_kind: ContainerKind,
    pub previous: NodeInfo,
    pub next: NodeInfo,
    pub range: Range<usize>,
    pub insertion_offset: usize,
}

struct Collector<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    boundaries: Vec<Boundary>,
}

pub fn collect_boundaries(source: &str) -> Result<Vec<Boundary>> {
    let parse = ast::SourceFile::parse(source, Edition::CURRENT);
    let errors = parse.errors();

    if !errors.is_empty() {
        return Err(anyhow!(
            "{}",
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    let file = parse.tree();

    let mut collector = Collector {
        source,
        line_starts: line_starts(source),
        boundaries: Vec::new(),
    };

    collector.collect(&file);

    // The CST walk is depth-first, while root item boundaries are collected
    // separately. Keep the public result in source order.
    collector
        .boundaries
        .sort_by_key(|boundary| (boundary.range.start, boundary.range.end));

    Ok(collector.boundaries)
}

impl<'a> Collector<'a> {
    fn collect(&mut self, file: &ast::SourceFile) {
        // SourceFile itself is the container for crate-root items.
        self.module_items(file.syntax());

        // Every nested syntactic container can be found directly in the CST.
        // There is no need to recursively enumerate every expression kind.
        for syntax in file.syntax().descendants() {
            if let Some(list) = ast::ItemList::cast(syntax.clone()) {
                self.module_items(list.syntax());
                continue;
            }

            if let Some(list) = ast::AssocItemList::cast(syntax.clone()) {
                self.associated_items(list.syntax());
                continue;
            }

            if let Some(list) = ast::ExternItemList::cast(syntax.clone()) {
                self.extern_items(list.syntax());
                continue;
            }

            if let Some(list) = ast::StmtList::cast(syntax) {
                self.statements(&list);
            }
        }
    }

    fn module_items(&mut self, parent: &SyntaxNode) {
        let nodes = parent
            .children()
            .filter_map(ast::Item::cast)
            .map(|item| self.item_node(&item))
            .collect();

        self.add_boundaries(ContainerKind::ItemList, nodes);
    }

    fn associated_items(&mut self, parent: &SyntaxNode) {
        let nodes = parent
            .children()
            .filter_map(ast::AssocItem::cast)
            .map(|item| self.associated_item_node(&item))
            .collect();

        self.add_boundaries(ContainerKind::ItemList, nodes);
    }

    fn extern_items(&mut self, parent: &SyntaxNode) {
        let nodes = parent
            .children()
            .filter_map(ast::ExternItem::cast)
            .map(|item| self.extern_item_node(&item))
            .collect();

        self.add_boundaries(ContainerKind::ItemList, nodes);
    }

    fn statements(&mut self, list: &ast::StmtList) {
        let mut nodes = Vec::new();

        // Ordinary statements are wrapped in Stmt nodes, while the final
        // tail expression is a direct Expr child of StmtList.
        for syntax in list.syntax().children() {
            if let Some(statement) = ast::Stmt::cast(syntax.clone()) {
                nodes.push(self.statement_node(&statement));
            } else if let Some(expression) = ast::Expr::cast(syntax) {
                nodes.push(self.tail_expression_node(&expression));
            }
        }

        self.add_boundaries(ContainerKind::StatementList, nodes);
    }

    fn add_boundaries(&mut self, parent_kind: ContainerKind, nodes: Vec<NodeInfo>) {
        for pair in nodes.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];

            let range = previous.trailing_range.end..next.leading_range.start;

            if range.start > range.end || range.end > self.source.len() {
                continue;
            }

            self.boundaries.push(Boundary {
                parent_kind,
                insertion_offset: insertion_offset(
                    self.source,
                    next.leading_range.start,
                    &self.line_starts,
                ),
                previous: previous.clone(),
                next: next.clone(),
                range,
            });
        }
    }

    fn item_node(&self, item: &ast::Item) -> NodeInfo {
        self.node_info(
            item.syntax(),
            if item_is_block_bodied(item) {
                NodeKind::BlockBodiedItem
            } else {
                NodeKind::Other
            },
            syntax_is_skipped(item.syntax()),
        )
    }

    fn associated_item_node(&self, item: &ast::AssocItem) -> NodeInfo {
        let syntax = item.syntax();

        self.node_info(
            syntax,
            if ast::Fn::can_cast(syntax.kind()) && ends_with_brace(syntax) {
                NodeKind::BlockBodiedItem
            } else {
                NodeKind::Other
            },
            syntax_is_skipped(syntax),
        )
    }

    fn extern_item_node(&self, item: &ast::ExternItem) -> NodeInfo {
        self.node_info(
            item.syntax(),
            NodeKind::Other,
            syntax_is_skipped(item.syntax()),
        )
    }

    fn statement_node(&self, statement: &ast::Stmt) -> NodeInfo {
        let (kind, skipped) = match statement {
            ast::Stmt::LetStmt(statement) => (
                NodeKind::LetStatement,
                syntax_is_skipped(statement.syntax()),
            ),

            ast::Stmt::ExprStmt(statement) => {
                let Some(expression) = statement.expr() else {
                    return self.node_info(
                        statement.syntax(),
                        NodeKind::Other,
                        syntax_is_skipped(statement.syntax()),
                    );
                };

                let kind = if expression_is_block_like(&expression) {
                    NodeKind::BlockExpressionStatement
                } else {
                    NodeKind::Other
                };

                (kind, syntax_is_skipped(expression.syntax()))
            }

            ast::Stmt::Item(item) => (NodeKind::Other, syntax_is_skipped(item.syntax())),
        };

        self.node_info(statement.syntax(), kind, skipped)
    }

    fn tail_expression_node(&self, expression: &ast::Expr) -> NodeInfo {
        // A tail expression has no following sibling statement, so its kind
        // cannot trigger any of the current "blank line after X" rules.
        self.node_info(
            expression.syntax(),
            NodeKind::Other,
            syntax_is_skipped(expression.syntax()),
        )
    }

    fn node_info(&self, syntax: &SyntaxNode, kind: NodeKind, skipped: bool) -> NodeInfo {
        let range = byte_range(syntax.text_range());

        NodeInfo {
            leading_range: leading_range(self.source, range.start, &self.line_starts),
            trailing_range: trailing_range(self.source, range.end),
            multiline: range_multiline(self.source, &range),
            range,
            kind,
            skipped,
        }
    }
}

fn item_is_block_bodied(item: &ast::Item) -> bool {
    let syntax = item.syntax();
    let kind = syntax.kind();

    let block_capable = ast::Fn::can_cast(kind)
        || ast::Impl::can_cast(kind)
        || ast::Trait::can_cast(kind)
        || ast::Module::can_cast(kind)
        || ast::Struct::can_cast(kind)
        || ast::Enum::can_cast(kind)
        || ast::Union::can_cast(kind)
        || ast::ExternBlock::can_cast(kind);

    block_capable && ends_with_brace(syntax)
}

fn expression_is_block_like(expression: &ast::Expr) -> bool {
    let kind = expression.syntax().kind();

    ast::IfExpr::can_cast(kind)
        || ast::MatchExpr::can_cast(kind)
        || ast::ForExpr::can_cast(kind)
        || ast::WhileExpr::can_cast(kind)
        || ast::LoopExpr::can_cast(kind)
        || ast::BlockExpr::can_cast(kind)
}

fn ends_with_brace(syntax: &SyntaxNode) -> bool {
    syntax.last_token().is_some_and(|token| token.text() == "}")
}

fn syntax_is_skipped(syntax: &SyntaxNode) -> bool {
    syntax
        .ancestors()
        .filter_map(ast::AnyHasAttrs::cast)
        .any(|owner| owner.attrs().any(|attr| is_rustfmt_skip(&attr)))
}

fn is_rustfmt_skip(attr: &ast::Attr) -> bool {
    let Some(path) = attr.path() else {
        return false;
    };

    let mut segments = path
        .segments()
        .filter_map(|segment| segment.name_ref())
        .map(|name| name.syntax().text().to_string());

    let first = segments.next();
    let second = segments.next();

    first.as_deref() == Some("rustfmt")
        && second.as_deref() == Some("skip")
        && segments.next().is_none()
}

fn byte_range(range: TextRange) -> Range<usize> {
    u32::from(range.start()) as usize..u32::from(range.end()) as usize
}

fn line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            source
                .bytes()
                .enumerate()
                .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
        )
        .collect()
}

fn range_multiline(source: &str, range: &Range<usize>) -> bool {
    source[range.clone()].contains('\n')
}

fn leading_range(source: &str, start: usize, starts: &[usize]) -> Range<usize> {
    let line = starts
        .partition_point(|&offset| offset <= start)
        .saturating_sub(1);

    let mut first = line;

    while first > 0 {
        let text = line_text(source, starts, first - 1);
        let trimmed = text.trim();

        if trimmed.is_empty() {
            break;
        }

        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            first -= 1;
        } else {
            break;
        }
    }

    starts[first]..start
}

fn trailing_range(source: &str, end: usize) -> Range<usize> {
    end..end
        + source[end..]
            .chars()
            .take_while(|c| *c != '\n')
            .map(char::len_utf8)
            .sum::<usize>()
}

fn line_text<'a>(source: &'a str, starts: &[usize], line: usize) -> &'a str {
    let end = starts.get(line + 1).copied().unwrap_or(source.len());

    source[starts[line]..end].trim_end_matches(['\r', '\n'])
}

fn insertion_offset(_source: &str, next_start: usize, starts: &[usize]) -> usize {
    let line = starts
        .partition_point(|&offset| offset <= next_start)
        .saturating_sub(1);

    starts[line]
}
