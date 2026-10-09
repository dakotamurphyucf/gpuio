//! Structural collection for a retained highlight declaration. The mounted
//! adapter supplies visibility and the exact installed document text; collection
//! never reparses Markdown or reads an editor's mutable/marked text.
use crate::{
    highlight_projection::{self as projection, Group, Projection, Run, RunKey, Source},
    tree::{Node, Tree},
};
use gpuio_protocol::{NodeId, v1::Kind};

/// Includes empty/ineligible nodes so a huge empty subtree has a finite cost.
pub const MAX_VISITED_NODES: usize = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentError {
    Pending,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    MissingScope,
    VisitLimit,
    Projection(projection::ProjectionError),
    Document(DocumentError),
}

/// Each inner vector is one logical displayed group, in stable source order.
/// The document adapter must supply bounded installed text, not raw Markdown or
/// frame visitation order. Empty vectors/fragments still consume source bounds.
/// It must invalidate its caller's cache on page/visibility/installed revision
/// changes. Fragment IDs are assigned consecutively, including empty fragments.
pub type DocumentGroups = Vec<Vec<Source>>;

struct Collector<'a, V, D> {
    tree: &'a Tree,
    visible: V,
    document: D,
    query_documents: bool,
    groups: Vec<Group>,
    bytes: usize,
    runs: usize,
    visited: usize,
}
impl<V, D> Collector<'_, V, D>
where
    V: FnMut(&Node) -> bool,
    D: FnMut(&Node) -> Result<DocumentGroups, DocumentError>,
{
    fn visit_budget(&mut self) -> Result<(), Error> {
        if self.visited == MAX_VISITED_NODES {
            return Err(Error::VisitLimit);
        }
        self.visited += 1;
        Ok(())
    }
    fn run(&mut self, node: NodeId, fragment: u32, source: Source) -> Result<Run, Error> {
        if self.runs == projection::MAX_RUNS {
            return Err(Error::Projection(projection::ProjectionError::RunLimit));
        }
        if source.len() > projection::MAX_SOURCE_BYTES - self.bytes {
            return Err(Error::Projection(projection::ProjectionError::ByteLimit));
        }
        self.runs += 1;
        self.bytes += source.len();
        Ok(Run {
            key: RunKey { node, fragment },
            source,
        })
    }
    fn group(&mut self, kind: projection::Kind, runs: Vec<Run>) -> Result<(), Error> {
        if self.groups.len() == projection::MAX_GROUPS {
            return Err(Error::Projection(projection::ProjectionError::GroupLimit));
        }
        self.groups.push(Group { kind, runs });
        Ok(())
    }
    fn flush(&mut self, runs: &mut Vec<Run>) -> Result<(), Error> {
        if !runs.is_empty() {
            self.group(projection::Kind::Ordinary, std::mem::take(runs))?;
        }
        Ok(())
    }
    // Tree validation already bounds depth to 128. Pending sibling runs are
    // flushed before descent; total retained output is checked before append.
    fn visit(&mut self, node: &Node, root: bool) -> Result<(), Error> {
        self.visit_budget()?;
        if (!root && node.highlight_scope.is_some()) || !(self.visible)(node) {
            return Ok(());
        }
        if node.kind == Kind::DocumentView {
            if !self.query_documents {
                return Ok(());
            }
            let groups = (self.document)(node).map_err(Error::Document)?;
            if groups.len() > projection::MAX_GROUPS - self.groups.len() {
                return Err(Error::Projection(projection::ProjectionError::GroupLimit));
            }
            let mut fragment = 0;
            for sources in groups {
                if sources.len() > projection::MAX_RUNS - self.runs {
                    return Err(Error::Projection(projection::ProjectionError::RunLimit));
                }
                let mut runs = Vec::with_capacity(sources.len());
                for source in sources {
                    runs.push(self.run(node.id, fragment, source)?);
                    fragment += 1;
                }
                self.group(projection::Kind::NativeDocument, runs)?;
            }
            return Ok(());
        }
        // Mirrors GPUIX's built-in text/div content rule. Widget metadata such
        // as an input value or panel title is not ordinary searchable content.
        if matches!(node.kind, Kind::Text | Kind::Container) && !node.text.is_empty() {
            let run = self.run(node.id, 0, Source::Text(node.text.clone()))?;
            self.group(projection::Kind::Ordinary, vec![run])?;
        }
        let mut pending = Vec::new();
        for id in node.children.iter() {
            let child = self.tree.get(*id).expect("validated retained child");
            if child.kind == Kind::Text
                && child.children.is_empty()
                && child.highlight_scope.is_none()
            {
                self.visit_budget()?;
                if (self.visible)(child) && !child.text.is_empty() {
                    pending.push(self.run(child.id, 0, Source::Text(child.text.clone()))?);
                }
            } else {
                self.flush(&mut pending)?;
                self.visit(child, false)?;
            }
        }
        self.flush(&mut pending)
    }
}

/// Collect a validated tree subtree. A nested declaration is an opaque boundary
/// even when empty. Hidden subtrees are omitted by the mounted visibility policy;
/// non-text siblings still break adjacent-text groups. No text is flattened.
/// Any failure discards the whole projection, never publishing partial counts.
pub fn collect(
    tree: &Tree,
    scope: NodeId,
    visible: impl FnMut(&Node) -> bool,
    document: impl FnMut(&Node) -> Result<DocumentGroups, DocumentError>,
) -> Result<Projection, Error> {
    let root = tree
        .get(scope)
        .filter(|node| node.highlight_scope.is_some())
        .ok_or(Error::MissingScope)?;
    let config = root.highlight_scope.as_ref().expect("checked scope");
    if config.0.is_empty() {
        return Projection::new(vec![]).map_err(Error::Projection);
    }
    let mut collector = Collector {
        tree,
        visible,
        document,
        query_documents: config.0.iter().any(|spec| spec.query.is_some()),
        groups: Vec::new(),
        bytes: 0,
        runs: 0,
        visited: 0,
    };
    collector.visit(root, true)?;
    Projection::new(collector.groups).map_err(Error::Projection)
}
