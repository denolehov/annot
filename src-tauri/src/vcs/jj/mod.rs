//! Jujutsu tier, backed by jj-lib.
//!
//! Three things make this more than "git with different words":
//!
//! 1. **The snapshot.** jj's working copy is a *commit* (`@`), and it only
//!    matches the filesystem because every `jj` command snapshots on startup.
//!    A tool that skips that step reviews whatever `@` happened to hold when
//!    the user last ran `jj` — silently stale, which for a review tool is a
//!    lie. So annot snapshots too, and accepts the consequence: one
//!    `snapshot working copy` entry in `jj op log`. It is the only write annot
//!    ever performs on a repository.
//!
//! 2. **Revsets, not revspecs.** `@-`, `trunk()`, a user's own aliases. They
//!    are resolved by jj's own engine against the user's own config, so a
//!    revset means here exactly what it means in `jj`. A revset naming one
//!    commit reviews that commit; one naming a contiguous stack reviews the
//!    whole stack as a single changeset (`trunk()..@` — the branch you'd open
//!    a PR for). Everything else is an error naming the candidates — never a
//!    silent "take the first", which would review *some* commit while the user
//!    believed it was *the* commit (divergent change ids make this real, and
//!    `mutable() & mine()` names several unrelated stacks at once).
//!
//! 3. **Conflicts are content.** In git a conflict means "you are mid-merge";
//!    the git tier errors. In jj a conflict is a committed, first-class object
//!    that a rebase can carry around, and "what did this rebase break?" is
//!    exactly the review a user wants. So conflicted files materialize into
//!    marker text — jj's own `diff` style, the same bytes `jj diff` shows —
//!    and flow through the pipeline as ordinary lines.

mod config;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{StreamExt as _, TryStreamExt as _};
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::conflicts::{
    materialize_merge_result_to_bytes, ConflictMarkerStyle, ConflictMaterializeOptions,
    MaterializedTreeValue,
};
use jj_lib::copies::{CopyOperation, CopyRecords};
use jj_lib::fileset;
use jj_lib::matchers::{EverythingMatcher, Matcher, NothingMatcher};
use jj_lib::merged_tree::MergedTree;
use jj_lib::object_id::ObjectId as _;
use jj_lib::repo::{ReadonlyRepo, Repo as _, StoreFactories};
use jj_lib::repo_path::{RepoPath, RepoPathUiConverter};
use jj_lib::revset::{
    self, RevsetAliasesMap, RevsetDiagnostics, RevsetExtensions, RevsetParseContext,
    RevsetStreamExt as _, RevsetWorkspaceContext, SymbolResolver, SymbolResolverExtension,
    UserRevsetExpression,
};
use jj_lib::settings::{HumanByteSize, UserSettings};
use jj_lib::working_copy::{SnapshotOptions, WorkingCopyFreshness};
use jj_lib::workspace::{default_working_copy_factories, Workspace};
use pollster::FutureExt as _;

use super::{tree_sort, BlobRef, DiffTarget, FileEntry, FileStatus, Prepared};
use crate::error::AnnotError;
use crate::source::{bytes_to_text, JjSource, Side, MAX_FILE_SIZE};

/// The working-copy revision, in jj's dialect. Fills an empty range side.
const AT: &str = "@";

fn jj_err(context: &str, e: impl std::fmt::Display) -> AnnotError {
    AnnotError::Diff(format!("{context}: {e}"))
}

/// Walk up from `cwd` looking for `.jj`. Mirrors jj-cli's `find_workspace_dir`
/// (jj-lib has no equivalent — `Workspace::load` demands the exact root).
///
/// `.jj` wins over a colocated `.git`, which is why this runs first: in a
/// colocated repo, git's HEAD and index are export artifacts of jj's state.
pub fn discover(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|path| path.join(".jj").is_dir())
        .map(Path::to_path_buf)
}

pub struct JjRepo {
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    settings: UserSettings,
    aliases: RevsetAliasesMap,
    root: PathBuf,
}

impl JjRepo {
    /// Load the workspace at `root` at the current operation.
    ///
    /// Loaded fresh, per session, and never cached across them: an MCP server
    /// is long-lived, and a jj repo moves under it every time the user runs a
    /// command. A cached `ReadonlyRepo` would serve a review of a repo state
    /// that no longer exists.
    pub fn load(root: &Path) -> Result<Self, AnnotError> {
        let settings = config::load_settings(&root.join(".jj").join("repo"))?;
        let workspace = Workspace::load(
            &settings,
            root,
            &StoreFactories::default(),
            &default_working_copy_factories(),
        )
        .map_err(|e| jj_err("failed to load jj workspace", e))?;
        let repo = workspace
            .repo_loader()
            .load_at_head()
            .block_on()
            .map_err(|e| jj_err("failed to load jj repo", e))?;
        let aliases = config::revset_aliases(&settings);
        Ok(Self {
            workspace,
            repo,
            settings,
            aliases,
            root: root.to_path_buf(),
        })
    }

    // -- revsets ------------------------------------------------------------

    fn with_parse_context<T>(
        &self,
        f: impl FnOnce(&RevsetParseContext) -> Result<T, AnnotError>,
    ) -> Result<T, AnnotError> {
        let path_converter = RepoPathUiConverter::Fs {
            cwd: self.root.clone(),
            base: self.root.clone(),
        };
        let extensions = RevsetExtensions::default();
        let fileset_aliases = Default::default();
        let context = RevsetParseContext {
            aliases_map: &self.aliases,
            local_variables: HashMap::new(),
            user_email: self.settings.user_email(),
            date_pattern_context: chrono::Local::now().into(),
            // Colocated repos have a synthetic "git" remote for the backing
            // git repo; jj hides it from bare bookmark names, and so must we.
            default_ignored_remote: Some(jj_lib::git::REMOTE_NAME_FOR_LOCAL_GIT_REPO),
            fileset_aliases_map: &fileset_aliases,
            extensions: &extensions,
            workspace: Some(RevsetWorkspaceContext {
                path_converter: &path_converter,
                workspace_name: self.workspace.workspace_name(),
            }),
        };
        f(&context)
    }

    /// Parse a revset into an expression, without evaluating it.
    ///
    /// Derived expressions (`roots`, `heads`, dag ranges) are then built with
    /// jj-lib's combinators on *this* value — never by interpolating the user's
    /// revset back into a format string, which would let `rev` smuggle syntax
    /// into an expression annot thought it was composing.
    fn parse(&self, revset_str: &str) -> Result<Arc<UserRevsetExpression>, AnnotError> {
        self.with_parse_context(|context| {
            let mut diagnostics = RevsetDiagnostics::new();
            revset::parse(&mut diagnostics, revset_str, context)
                .map_err(|e| jj_err(&format!("failed to parse revset '{revset_str}'"), e))
        })
    }

    /// Evaluate an expression to at most `limit` commits.
    fn evaluate(
        &self,
        expression: &Arc<UserRevsetExpression>,
        revset_str: &str,
        limit: usize,
    ) -> Result<Vec<Commit>, AnnotError> {
        let repo = self.repo.as_ref();
        let symbol_resolver =
            SymbolResolver::new(repo, &([] as [&Box<dyn SymbolResolverExtension>; 0]));
        let resolved = expression
            .resolve_user_expression(repo, &symbol_resolver)
            .map_err(|e| jj_err(&format!("failed to resolve '{revset_str}'"), e))?;
        resolved
            .evaluate(repo)
            .map_err(|e| jj_err(&format!("failed to evaluate '{revset_str}'"), e))?
            .stream()
            .commits(repo.store())
            .take(limit)
            .try_collect()
            .block_on()
            .map_err(|e| jj_err(&format!("failed to evaluate '{revset_str}'"), e))
    }

    /// Whether an expression evaluates to nothing — used to prove set equality
    /// without materializing either side (`::@` is a legitimate, huge revset).
    fn is_empty(
        &self,
        expression: &Arc<UserRevsetExpression>,
        revset_str: &str,
    ) -> Result<bool, AnnotError> {
        Ok(self.evaluate(expression, revset_str, 1)?.is_empty())
    }

    /// Resolve a revset to *exactly one* commit. Used where a single endpoint
    /// is the only thing that can mean anything — the two sides of a range.
    ///
    /// Zero and many are both errors. "Many" is not a corner case: a divergent
    /// change id resolves to several commits, and picking the first would
    /// review a commit the user did not name.
    fn resolve(&self, revset_str: &str) -> Result<Commit, AnnotError> {
        let revset_str = if revset_str.is_empty() {
            AT
        } else {
            revset_str
        };
        let expression = self.parse(revset_str)?;
        // Four is enough to say "many" and name a few of them.
        let commits = self.evaluate(&expression, revset_str, 4)?;
        match commits.as_slice() {
            [commit] => Ok(commit.clone()),
            [] => Err(self.empty_revset_err(revset_str)),
            many => Err(self.ambiguous_err(revset_str, many, "name one of them")),
        }
    }

    fn empty_revset_err(&self, revset_str: &str) -> AnnotError {
        AnnotError::Diff(format!(
            "revset '{revset_str}' didn't resolve to any revision"
        ))
    }

    fn ambiguous_err(&self, revset_str: &str, many: &[Commit], advice: &str) -> AnnotError {
        let candidates = many
            .iter()
            .map(|c| self.change_id(c))
            .collect::<Vec<_>>()
            .join(", ");
        AnnotError::Diff(format!(
            "revset '{revset_str}' resolved to {}{} revisions ({candidates}{}) — {advice}",
            many.len(),
            if many.len() > 3 { "+" } else { "" },
            if many.len() > 3 { ", ..." } else { "" },
        ))
    }

    /// Resolve a revset into something reviewable as *one* diff.
    ///
    /// One commit is the obvious case. Several are reviewable only when they
    /// form a single contiguous stack — one root, one head, and nothing in
    /// between that the revset left out — because only then does a single
    /// before/after exist: the root's base, and the head's tree. That is what
    /// makes `annot diff 'trunk()..@'` (or an alias for it) mean "review my
    /// branch".
    ///
    /// Anything else errors, exactly as before. `mutable() & mine()` picks out
    /// several unrelated stacks; there is no diff of that, and inventing one
    /// would show a diff spanning branches the user never asked about.
    fn resolve_reviewable(&self, revset_str: &str) -> Result<Reviewable, AnnotError> {
        let revset_str = if revset_str.is_empty() {
            AT
        } else {
            revset_str
        };
        let expression = self.parse(revset_str)?;
        let commits = self.evaluate(&expression, revset_str, 4)?;
        match commits.as_slice() {
            [commit] => return Ok(Reviewable::Single(commit.clone())),
            [] => return Err(self.empty_revset_err(revset_str)),
            _ => {}
        }

        let advice = "name a single revision, or a contiguous stack of them";
        let roots_expr = expression.roots();
        let heads_expr = expression.heads();
        let roots = self.evaluate(&roots_expr, revset_str, 2)?;
        let heads = self.evaluate(&heads_expr, revset_str, 2)?;
        let ([root], [head]) = (roots.as_slice(), heads.as_slice()) else {
            // Several roots or several heads: disjoint stacks, not one chain.
            return Err(self.ambiguous_err(revset_str, &commits, advice));
        };

        // One root and one head are not enough on their own — the revset could
        // still be a *subset* of the chain between them (`@ | @---`, say), and
        // diffing root..head would then silently include commits it excluded.
        // Prove set equality both ways, using is_empty so neither side has to
        // be materialized.
        let span = roots_expr.dag_range_to(&heads_expr);
        let complete = self.is_empty(&span.minus(&expression), revset_str)?
            && self.is_empty(&expression.minus(&span), revset_str)?;
        if !complete {
            return Err(self.ambiguous_err(revset_str, &commits, advice));
        }

        Ok(Reviewable::Stack {
            root: root.clone(),
            head: head.clone(),
        })
    }

    /// A commit's change id, in the form jj prints and accepts: reverse-hex
    /// (`kmxyzqrs`), not raw hex. Change ids lead because they survive
    /// rewrites — an agent that quotes one back can still act on it after an
    /// amend, where a commit id may already be dangling.
    fn change_id(&self, commit: &Commit) -> String {
        let hex = jj_lib::hex_util::encode_reverse_hex(commit.change_id().as_bytes());
        hex[..CHANGE_ID_LEN.min(hex.len())].to_string()
    }

    // -- snapshot -----------------------------------------------------------

    /// Snapshot the working copy into `@`, exactly as every `jj` command does
    /// on startup, and reload the repo at the resulting operation.
    ///
    /// This is annot's only write. It appears in `jj op log` as
    /// `snapshot working copy` — the same entry `jj status` would leave.
    fn snapshot(&mut self) -> Result<(), AnnotError> {
        let auto_track = self.auto_tracking_matcher()?;
        let max_new_file_size = self
            .settings
            .get_value_with("snapshot.max-new-file-size", HumanByteSize::try_from)
            .map(|HumanByteSize(size)| size)
            .unwrap_or(MAX_FILE_SIZE);
        let options = SnapshotOptions {
            base_ignores: jj_lib::gitignore::GitIgnoreFile::empty(),
            progress: None,
            start_tracking_matcher: auto_track.as_ref(),
            force_tracking_matcher: &NothingMatcher,
            max_new_file_size: if max_new_file_size == 0 {
                u64::MAX
            } else {
                max_new_file_size
            },
        };

        let workspace_name = self.workspace.workspace_name().to_owned();
        let mut locked_ws = self
            .workspace
            .start_working_copy_mutation()
            .block_on()
            .map_err(|e| jj_err("failed to lock jj working copy", e))?;

        // Another jj process may have moved @ since we loaded the repo. Rebase
        // our view onto its operation before snapshotting, or we'd snapshot
        // against a base commit that is no longer current.
        let (repo, wc_commit) =
            match freshen(locked_ws.locked_wc(), self.repo.clone(), &workspace_name)? {
                Some(pair) => pair,
                // The workspace was forgotten out from under us; nothing to
                // snapshot into.
                None => return Ok(()),
            };
        self.repo = repo;

        let (new_tree, _stats) = locked_ws
            .locked_wc()
            .snapshot(&options)
            .block_on()
            .map_err(|e| jj_err("failed to snapshot jj working copy", e))?;

        if new_tree.tree_ids_and_labels() == wc_commit.tree().tree_ids_and_labels() {
            // Clean working copy: no new operation, just release the lock at
            // the operation we already have.
            let op_id = self.repo.op_id().clone();
            return locked_ws
                .finish(op_id)
                .block_on()
                .map_err(|e| jj_err("failed to release jj working copy", e));
        }

        let mut tx = self.repo.start_transaction();
        tx.set_is_snapshot(true);
        tx.set_workspace_name(&workspace_name);
        let mut_repo = tx.repo_mut();
        let new_wc_commit = mut_repo
            .rewrite_commit(&wc_commit)
            .set_tree(new_tree)
            .write()
            .block_on()
            .map_err(|e| jj_err("failed to write jj working-copy commit", e))?;
        mut_repo
            .set_wc_commit(workspace_name, new_wc_commit.id().clone())
            .map_err(|e| jj_err("failed to update jj working copy", e))?;
        // Mandatory before commit: `Transaction::commit` asserts there are no
        // pending rewrites.
        mut_repo
            .rebase_descendants()
            .block_on()
            .map_err(|e| jj_err("failed to rebase descendants", e))?;

        let repo = tx
            .commit("snapshot working copy")
            .block_on()
            .map_err(|e| jj_err("failed to commit jj snapshot", e))?;
        locked_ws
            .finish(repo.op_id().clone())
            .block_on()
            .map_err(|e| jj_err("failed to release jj working copy", e))?;
        self.repo = repo;
        Ok(())
    }

    /// `snapshot.auto-track` decides which *untracked* files become part of
    /// `@`. Reading it from config (rather than tracking everything) is the
    /// difference between honoring the user's rules and inventing our own.
    fn auto_tracking_matcher(&self) -> Result<Box<dyn Matcher>, AnnotError> {
        let pattern = self
            .settings
            .get_string("snapshot.auto-track")
            .unwrap_or_else(|_| "all()".to_string());
        // auto-track patterns are always repo-root-relative, so the converter
        // gets empty cwd/base — same as jj.
        let path_converter = RepoPathUiConverter::Fs {
            cwd: PathBuf::new(),
            base: PathBuf::new(),
        };
        let aliases = Default::default();
        let context = fileset::FilesetParseContext {
            aliases_map: &aliases,
            path_converter: &path_converter,
        };
        let mut diagnostics = fileset::FilesetDiagnostics::new();
        let expression = fileset::parse(&mut diagnostics, &pattern, &context)
            .map_err(|e| jj_err("invalid `snapshot.auto-track` pattern", e))?;
        Ok(expression.to_matcher())
    }

    // -- enumeration --------------------------------------------------------

    /// Enumerate changed files for `target`, materializing both sides.
    pub fn enumerate(
        &mut self,
        target: &DiffTarget,
        pathspecs: &[String],
    ) -> Result<Prepared, AnnotError> {
        if matches!(target, DiffTarget::Staged) {
            return Err(AnnotError::Diff(
                "jj has no staging area — use the working_copy target, or name a revision".into(),
            ));
        }

        // Every jj command snapshots on startup; so does annot. Without it a
        // `Revision{"@"}` review would show the last state jj happened to
        // record, not what is on disk.
        self.snapshot()?;

        let matcher = self.path_matcher(pathspecs)?;
        let comparison = self.comparison(target)?;
        let Comparison {
            from_tree,
            to_tree,
            copy_sources,
            to_id,
            label,
        } = comparison;
        let copy_records = self.copy_records(&copy_sources, &to_id);

        let entries: Vec<_> = from_tree
            .diff_stream_with_copies(&to_tree, matcher.as_ref(), &copy_records)
            .collect()
            .block_on();

        let mut file_entries = Vec::with_capacity(entries.len());
        let mut texts: HashMap<(String, Side), Option<Arc<str>>> = HashMap::new();
        let mut conflicted: HashSet<String> = HashSet::new();
        for entry in entries {
            let values = entry
                .values
                .map_err(|e| jj_err("failed to read jj tree diff", e))?;
            let target = &entry.path.target;
            let source = entry.path.source.as_ref().map(|(path, _)| path);
            let source_path = source.unwrap_or(target);

            let before = self.materialize(source_path, &values.before, &from_tree)?;
            let after = self.materialize(target, &values.after, &to_tree)?;
            for (path, side) in [(source_path, &before), (target, &after)] {
                if side.is_conflict() {
                    conflicted.insert(path.as_internal_file_string().to_string());
                }
            }

            if let Some(file_entry) = build_entry(
                target.as_internal_file_string(),
                source.map(|p| p.as_internal_file_string()),
                entry.path.copy_operation(),
                &before,
                &after,
                &mut texts,
            ) {
                file_entries.push(file_entry);
            }
        }

        Ok(Prepared {
            entries: tree_sort(file_entries),
            source: Arc::new(JjSource::new(texts, conflicted)),
            label: Some(label),
        })
    }

    /// Which trees to compare, and what to call the comparison.
    fn comparison(&self, target: &DiffTarget) -> Result<Comparison, AnnotError> {
        match target {
            DiffTarget::Staged => unreachable!("rejected in enumerate"),
            // The working copy IS a commit in jj — `@` — so this is the same
            // code path as any other revision. The snapshot already ran.
            DiffTarget::WorkingCopy => self.revision_comparison(AT),
            DiffTarget::Revision { rev } => self.revision_comparison(rev),
            DiffTarget::Range {
                from,
                to,
                merge_base,
            } => {
                let to_commit = self.resolve(to)?;
                let from_commit = if *merge_base {
                    let a = self.parse(if from.is_empty() { AT } else { from })?;
                    let b = self.parse(if to.is_empty() { AT } else { to })?;
                    // heads(::a & ::b), composed rather than string-interpolated.
                    // Divergent history can yield several merge bases; this
                    // errors rather than silently picking one.
                    let base = a.ancestors().intersection(&b.ancestors()).heads();
                    let label = format!("merge base of '{from}' and '{to}'");
                    match self.evaluate(&base, &label, 2)?.as_slice() {
                        [commit] => commit.clone(),
                        [] => return Err(self.empty_revset_err(&label)),
                        many => return Err(self.ambiguous_err(&label, many, "name one of them")),
                    }
                } else {
                    self.resolve(from)?
                };
                Ok(Comparison {
                    from_tree: from_commit.tree(),
                    to_tree: to_commit.tree(),
                    copy_sources: vec![from_commit.id().clone()],
                    to_id: to_commit.id().clone(),
                    label: format!(
                        "{}{}{}",
                        if from.is_empty() { AT } else { from },
                        if *merge_base { "..." } else { ".." },
                        if to.is_empty() { AT } else { to },
                    ),
                })
            }
        }
    }

    /// A revset naming one commit, or one contiguous stack of them.
    ///
    /// Single: the commit vs its *auto-merged* parents — `jj diff -r`'s
    /// convention. A merge commit shows what the merge itself changed; git's
    /// first-parent rule would instead show the whole other branch.
    /// `parent_tree` also yields the empty tree for a root commit, so that case
    /// needs no branch.
    ///
    /// Stack: the base the stack sits on vs its head — every commit in the
    /// stack, squashed into one review. `trunk()..@` is the branch you'd open a
    /// PR for.
    fn revision_comparison(&self, rev: &str) -> Result<Comparison, AnnotError> {
        let (base, head, label) = match self.resolve_reviewable(rev)? {
            Reviewable::Single(commit) => {
                let label = format!(
                    "{} · {}",
                    if rev.is_empty() { AT } else { rev },
                    self.change_id(&commit)
                );
                (commit.clone(), commit, label)
            }
            Reviewable::Stack { root, head } => {
                // Counted, not listed: the point of reviewing a stack is that
                // it's one changeset, but the label shouldn't lie about how
                // many commits went into it.
                let count = self.evaluate(&self.parse(rev)?, rev, usize::MAX)?.len();
                let label = format!(
                    "{} · {} commits · {}..{}",
                    if rev.is_empty() { AT } else { rev },
                    count,
                    self.change_id(&root),
                    self.change_id(&head),
                );
                (root, head, label)
            }
        };

        // The stack's base is the *root's* parents — for a single commit the
        // root and head are the same commit, so this is the same expression.
        let from_tree = base
            .parent_tree(self.repo.as_ref())
            .block_on()
            .map_err(|e| jj_err("failed to load parent tree", e))?;
        Ok(Comparison {
            from_tree,
            to_tree: head.tree(),
            copy_sources: base.parent_ids().to_vec(),
            to_id: head.id().clone(),
            label,
        })
    }

    /// Rename/copy records for the comparison, when the backend tracks them.
    ///
    /// A backend that doesn't track copies yields none, and renames render as
    /// an add plus a delete — which is what `jj diff` shows too.
    fn copy_records(&self, sources: &[CommitId], to: &CommitId) -> CopyRecords {
        let mut records = CopyRecords::default();
        for source in sources {
            let Ok(stream) = self.repo.store().get_copy_records(None, source, to) else {
                continue;
            };
            let collected: Vec<_> = stream.try_collect().block_on().unwrap_or_default();
            records.add_records(collected);
        }
        records
    }

    /// jj's filesets stand in for git's pathspecs. `src/` parses as a bare
    /// path, `glob:"*.rs"` as an expression — the dialect the user already
    /// types into `jj diff`.
    fn path_matcher(&self, pathspecs: &[String]) -> Result<Box<dyn Matcher>, AnnotError> {
        if pathspecs.is_empty() {
            return Ok(Box::new(EverythingMatcher));
        }
        let path_converter = RepoPathUiConverter::Fs {
            cwd: self.root.clone(),
            base: self.root.clone(),
        };
        let aliases = Default::default();
        let context = fileset::FilesetParseContext {
            aliases_map: &aliases,
            path_converter: &path_converter,
        };
        let mut expressions = Vec::new();
        for spec in pathspecs {
            let mut diagnostics = fileset::FilesetDiagnostics::new();
            let expression = fileset::parse_maybe_bare(&mut diagnostics, spec, &context)
                .map_err(|e| jj_err(&format!("invalid path '{spec}'"), e))?;
            expressions.push(expression);
        }
        Ok(fileset::FilesetExpression::union_all(expressions).to_matcher())
    }

    // -- content ------------------------------------------------------------

    /// One side of one file, as annot's pipeline wants it.
    fn materialize(
        &self,
        path: &RepoPath,
        value: &jj_lib::merge::MergedTreeValue,
        tree: &MergedTree,
    ) -> Result<Materialized, AnnotError> {
        let materialized = jj_lib::conflicts::materialize_tree_value(
            self.repo.store(),
            path,
            value.clone(),
            tree.labels(),
        )
        .block_on()
        .map_err(|e| jj_err("failed to read jj file", e))?;

        Ok(match materialized {
            MaterializedTreeValue::Absent => Materialized::Absent,
            MaterializedTreeValue::File(mut file) => {
                let bytes = file
                    .read_all(path)
                    .block_on()
                    .map_err(|e| jj_err("failed to read jj file", e))?;
                Materialized::File {
                    text: to_text(bytes),
                    conflict: false,
                }
            }
            MaterializedTreeValue::FileConflict(file) => {
                let options = ConflictMaterializeOptions {
                    // jj's own default: `%%%%%%%` diff-style markers, which
                    // carry more than git's `<<<<<<<` — each side shown as a
                    // diff from the base. These are the exact bytes `jj diff`
                    // puts in front of the user.
                    marker_style: ConflictMarkerStyle::Diff,
                    marker_len: None,
                    merge: self.repo.store().merge_options().clone(),
                };
                // `file.labels` — the *simplified* labels that match
                // `file.contents`, not the tree's unsimplified ones.
                let bytes =
                    materialize_merge_result_to_bytes(&file.contents, &file.labels, &options);
                Materialized::File {
                    text: to_text(bytes.into()),
                    conflict: true,
                }
            }
            // Symlinks, submodules, trees, and non-file conflicts exist but
            // have no reviewable text — the same `Ok(None)` capability signal
            // GixSource gives for binary content.
            MaterializedTreeValue::Symlink { .. } => Materialized::Opaque { symlink: true },
            MaterializedTreeValue::AccessDenied(_)
            | MaterializedTreeValue::OtherConflict { .. }
            | MaterializedTreeValue::GitSubmodule(_)
            | MaterializedTreeValue::Tree(_) => Materialized::Opaque { symlink: false },
        })
    }
}

/// How long a change id we show. jj's own shortest-unique-prefix logic needs
/// an index we don't otherwise build; 8 is unambiguous in practice and still
/// pasteable back into `jj`.
const CHANGE_ID_LEN: usize = 8;

/// What a revset named, once we know it can be reviewed as one diff.
enum Reviewable {
    /// Exactly one commit.
    Single(Commit),
    /// One contiguous stack — reviewed as a single changeset, base to tip.
    Stack { root: Commit, head: Commit },
}

/// A resolved tree-vs-tree comparison.
struct Comparison {
    from_tree: MergedTree,
    to_tree: MergedTree,
    /// Commits to ask the backend for copy records against — a merge commit
    /// has several parents, each a possible rename source.
    copy_sources: Vec<CommitId>,
    to_id: CommitId,
    label: String,
}

/// A materialized side of a file.
enum Materialized {
    /// The side doesn't exist (added or deleted file).
    Absent,
    /// The side is a file. `text` is `None` when it exists but isn't
    /// reviewable — binary, oversize, or not UTF-8. `conflict` marks text
    /// that is materialized conflict markers rather than a plain blob.
    File {
        text: Option<Arc<str>>,
        conflict: bool,
    },
    /// Exists, but has no text: symlink, submodule, tree, non-file conflict.
    Opaque { symlink: bool },
}

impl Materialized {
    fn exists(&self) -> bool {
        !matches!(self, Materialized::Absent)
    }
    fn text(&self) -> Option<Arc<str>> {
        match self {
            Materialized::File { text, .. } => text.clone(),
            _ => None,
        }
    }
    fn is_conflict(&self) -> bool {
        matches!(self, Materialized::File { conflict: true, .. })
    }
    fn is_symlink(&self) -> bool {
        matches!(self, Materialized::Opaque { symlink: true })
    }
}

fn to_text(bytes: Vec<u8>) -> Option<Arc<str>> {
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return None;
    }
    bytes_to_text(bytes)
}

/// Turn a materialized before/after pair into a `FileEntry`, recording both
/// sides' texts. `None` when the file is absent on both sides.
fn build_entry(
    target_path: &str,
    source_path: Option<&str>,
    copy_op: Option<CopyOperation>,
    before: &Materialized,
    after: &Materialized,
    texts: &mut HashMap<(String, Side), Option<Arc<str>>>,
) -> Option<FileEntry> {
    let old_path = source_path.unwrap_or(target_path);
    let status = match (before.exists(), after.exists()) {
        (false, false) => return None,
        (false, true) => FileStatus::Added,
        (true, false) => FileStatus::Deleted,
        (true, true) => match copy_op {
            Some(CopyOperation::Copy) => FileStatus::Copied,
            Some(CopyOperation::Rename) => FileStatus::Renamed {
                similarity: similarity(before, after),
            },
            // A file that turned into a symlink (or back) is git's `T`.
            None if before.is_symlink() != after.is_symlink() => FileStatus::TypeChanged,
            None => FileStatus::Modified,
        },
    };

    // The oid carries only existence here; `JjSource` holds the real content
    // (a conflicted side has no single id to name it by).
    let old_oid = before.exists().then(|| format!("jj:old:{old_path}"));
    let new_oid = after
        .exists()
        .then(|| BlobRef::Oid(format!("jj:new:{target_path}")));

    if before.exists() {
        texts.insert((old_path.to_string(), Side::Old), before.text());
    }
    if after.exists() {
        texts.insert((target_path.to_string(), Side::New), after.text());
    }

    Some(FileEntry {
        status,
        old_path: before.exists().then(|| old_path.to_string()),
        new_path: after.exists().then(|| target_path.to_string()),
        old_oid,
        new_oid,
    })
}

/// Rename similarity, which jj's copy records don't carry. Best effort: a
/// content ratio when both sides are text, else the detection threshold.
fn similarity(before: &Materialized, after: &Materialized) -> u8 {
    match (before.text(), after.text()) {
        (Some(old), Some(new)) => {
            (similar::TextDiff::from_lines(old.as_ref(), new.as_ref()).ratio() * 100.0).round()
                as u8
        }
        _ => 50,
    }
}

/// Reconcile our loaded repo with the working copy's operation before
/// snapshotting. Mirrors jj-cli's `handle_stale_working_copy`.
fn freshen(
    locked_wc: &mut dyn jj_lib::working_copy::LockedWorkingCopy,
    repo: Arc<ReadonlyRepo>,
    workspace_name: &jj_lib::ref_name::WorkspaceName,
) -> Result<Option<(Arc<ReadonlyRepo>, Commit)>, AnnotError> {
    let Some(wc_commit_id) = repo.view().get_wc_commit_id(workspace_name).cloned() else {
        return Ok(None); // workspace was forgotten
    };
    let wc_commit = load_commit(&repo, &wc_commit_id)?;

    match WorkingCopyFreshness::check_stale(locked_wc, &wc_commit, &repo)
        .block_on()
        .map_err(|e| jj_err("failed to check jj working-copy freshness", e))?
    {
        WorkingCopyFreshness::Fresh => Ok(Some((repo, wc_commit))),
        // Another jj process moved @ after we loaded the repo — adopt its
        // operation rather than snapshotting onto a stale base.
        WorkingCopyFreshness::Updated(operation) => {
            let repo = repo
                .reload_at(&operation)
                .block_on()
                .map_err(|e| jj_err("failed to reload jj repo", e))?;
            let Some(wc_commit_id) = repo.view().get_wc_commit_id(workspace_name).cloned() else {
                return Ok(None);
            };
            let wc_commit = load_commit(&repo, &wc_commit_id)?;
            Ok(Some((repo, wc_commit)))
        }
        WorkingCopyFreshness::WorkingCopyStale => Err(AnnotError::Diff(
            "the jj working copy is stale — run `jj workspace update-stale`".into(),
        )),
        WorkingCopyFreshness::SiblingOperation => Err(AnnotError::Diff(
            "the jj working copy was changed by another operation — \
             run `jj workspace update-stale`"
                .into(),
        )),
    }
}

fn load_commit(repo: &ReadonlyRepo, id: &CommitId) -> Result<Commit, AnnotError> {
    repo.store()
        .get_commit(id)
        .map_err(|e| jj_err("failed to load commit", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{jj, jj_colocated_repo, jj_repo};
    use crate::vcs::{self, Backend};
    use std::fs;

    fn open(dir: &Path) -> JjRepo {
        JjRepo::load(&discover(dir).expect("no .jj found")).unwrap()
    }

    fn enumerate(dir: &Path, target: &DiffTarget) -> Vec<FileEntry> {
        open(dir).enumerate(target, &[]).unwrap().entries
    }

    fn text(dir: &Path, target: &DiffTarget, path: &str, side: Side) -> Option<String> {
        open(dir)
            .enumerate(target, &[])
            .unwrap()
            .source
            .full_text(path, side)
            .unwrap()
            .map(|t| t.as_ref().to_string())
    }

    /// `Result::unwrap_err` needs `Ok: Debug`, and a boxed `FileSource` isn't.
    fn enumerate_err(dir: &Path, target: &DiffTarget) -> AnnotError {
        match open(dir).enumerate(target, &[]) {
            Err(e) => e,
            Ok(p) => panic!("expected an error, got {:?}", p.entries),
        }
    }

    fn find<'a>(entries: &'a [FileEntry], path: &str) -> &'a FileEntry {
        entries
            .iter()
            .find(|e| e.new_path.as_deref() == Some(path) || e.old_path.as_deref() == Some(path))
            .unwrap_or_else(|| panic!("no entry for {path}: {entries:?}"))
    }

    /// The bug that started this: a jj repo with no `.git` anywhere. gix's
    /// discover found nothing and `annot diff` died with "could not find a git
    /// repository".
    #[test]
    fn non_colocated_repo_is_reviewable() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();

        assert!(dir.path().join(".jj").is_dir());
        assert!(!dir.path().join(".git").exists());
        assert!(matches!(
            vcs::open(dir.path()).unwrap(),
            Backend::Jj(_) // not Git — and not an error
        ));

        let entries = enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert_eq!(find(&entries, "a.txt").status, FileStatus::Added);
    }

    /// The staleness kill: jj only records the working copy when a jj command
    /// runs. Nothing has run here since the write — annot must snapshot for
    /// itself, or it would show an empty diff.
    #[test]
    fn working_copy_sees_edits_no_jj_command_has_recorded() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        jj(dir.path(), &["describe", "-m", "base"]);
        jj(dir.path(), &["new"]);

        // Edit with no jj command afterwards — @ still holds the old tree.
        fs::write(dir.path().join("a.txt"), "alpha\nbeta\n").unwrap();
        fs::write(dir.path().join("fresh.txt"), "new file\n").unwrap();

        let entries = enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert_eq!(find(&entries, "a.txt").status, FileStatus::Modified);
        assert_eq!(find(&entries, "fresh.txt").status, FileStatus::Added);
        assert_eq!(
            text(dir.path(), &DiffTarget::WorkingCopy, "a.txt", Side::New).as_deref(),
            Some("alpha\nbeta\n")
        );
    }

    /// The snapshot is a real write, and it is the *only* one: reviewing must
    /// leave exactly one new operation, and never rewrite history.
    #[test]
    fn snapshot_adds_one_operation_and_a_clean_review_adds_none() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        jj(dir.path(), &["describe", "-m", "base"]);

        let ops = |dir: &Path| {
            jj(
                dir,
                &["op", "log", "--no-graph", "-T", r#"id.short() ++ "\n""#],
            )
            .lines()
            .count()
        };
        let before = ops(dir.path());

        // A dirty working copy: one snapshot operation.
        fs::write(dir.path().join("a.txt"), "alpha\nbeta\n").unwrap();
        enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert_eq!(ops(dir.path()), before + 1);

        // A clean one: nothing to record, so no operation at all.
        let after_snapshot = ops(dir.path());
        enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert_eq!(ops(dir.path()), after_snapshot);
    }

    /// `annot diff <change-id>` — the ask that forced a `..` range before.
    #[test]
    fn revision_diffs_against_its_parent() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        jj(dir.path(), &["describe", "-m", "base"]);
        jj(dir.path(), &["new"]);
        fs::write(dir.path().join("b.txt"), "bravo\n").unwrap();
        jj(dir.path(), &["describe", "-m", "second"]);
        let change = jj(
            dir.path(),
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        );

        let target = DiffTarget::Revision {
            rev: change.clone(),
        };
        let entries = enumerate(dir.path(), &target);
        assert_eq!(entries.len(), 1);
        assert_eq!(find(&entries, "b.txt").status, FileStatus::Added);

        // `@-` resolves too — a revset, not a revspec.
        let parent = enumerate(dir.path(), &DiffTarget::Revision { rev: "@-".into() });
        assert_eq!(find(&parent, "a.txt").status, FileStatus::Added);
    }

    /// A stack fixture: base, then three commits on top of it, plus a second
    /// unrelated stack rooted at base. Returns (base, tip of the main stack).
    ///
    /// ```text
    ///   c ── tip of the stack under review
    ///   b
    ///   a        d ── a sibling stack, deliberately NOT part of it
    ///    \      /
    ///     base
    /// ```
    fn stack_repo() -> (tempfile::TempDir, String, String) {
        let dir = jj_repo();
        let p = dir.path();
        fs::write(p.join("base.txt"), "base\n").unwrap();
        jj(p, &["describe", "-m", "base"]);
        let base = jj(
            p,
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        );

        for name in ["a", "b", "c"] {
            jj(p, &["new"]);
            fs::write(p.join(format!("{name}.txt")), format!("{name}\n")).unwrap();
            jj(p, &["describe", "-m", name]);
        }
        let tip = jj(
            p,
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        );
        jj(p, &["bookmark", "create", "-r", "@", "tip"]);

        // A sibling stack off the same base — the thing that must NOT get
        // swept into a review of the main one.
        jj(p, &["new", &base]);
        fs::write(p.join("d.txt"), "d\n").unwrap();
        jj(p, &["describe", "-m", "d"]);
        jj(p, &["bookmark", "create", "-r", "@", "sibling"]);

        (dir, base, tip)
    }

    /// A revset naming a contiguous stack reviews as ONE changeset: the base
    /// the stack sits on vs its tip. This is `annot diff 'trunk()..@'` — the
    /// branch you'd open a PR for — and it's what makes revset aliases
    /// first-class instead of second-class beside `..`.
    #[test]
    fn a_contiguous_stack_reviews_as_one_changeset() {
        let (dir, base, _tip) = stack_repo();
        let p = dir.path();

        let target = DiffTarget::Revision {
            rev: format!("{base}..tip"),
        };
        let prepared = open(p).enumerate(&target, &[]).unwrap();

        // All three commits' files, as one diff — and NOT the sibling's.
        let paths: Vec<&str> = prepared
            .entries
            .iter()
            .map(|e| e.new_path.as_deref().unwrap())
            .collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "c.txt"]);
        assert!(prepared
            .entries
            .iter()
            .all(|e| e.status == FileStatus::Added));

        // base.txt is the stack's foundation, not part of it.
        assert!(!paths.contains(&"base.txt"));

        // The label says it's a stack, and how many commits went into it.
        let label = prepared.label.unwrap();
        assert!(label.contains("3 commits"), "{label}");
    }

    /// The trap this rule exists for. `mutable() & mine()` names several
    /// unrelated stacks; there is no single diff of that. The naive check
    /// ("does the set equal roots::heads?") passes here — because with several
    /// roots that range is just the union of the disjoint stacks — so the real
    /// rule is one root AND one head.
    #[test]
    fn disjoint_stacks_are_not_reviewable() {
        let (dir, base, tip) = stack_repo();
        let p = dir.path();

        // Both stacks at once: two heads, two roots.
        let target = DiffTarget::Revision {
            rev: format!("({base}..tip) | ({base}..sibling)"),
        };
        let err = enumerate_err(p, &target);
        assert!(err.to_string().contains("resolved to"), "{err}");
        assert!(
            err.to_string().contains("contiguous stack"),
            "the error should say what WOULD work: {err}"
        );
        // It must never have silently reviewed one of them.
        assert!(!err.to_string().contains("no changes"), "{err}");
        drop(tip);
    }

    /// One root and one head are still not enough: a revset can name a
    /// *subset* of the chain between them. Diffing root..head would then
    /// include the commit the revset deliberately left out.
    #[test]
    fn a_gappy_subset_of_a_chain_is_not_reviewable() {
        let (dir, _base, _tip) = stack_repo();
        let p = dir.path();

        // `tip | tip--` — one root (tip--), one head (tip), but the commit
        // between them is excluded. There is no honest diff for this.
        let target = DiffTarget::Revision {
            rev: "tip | tip--".into(),
        };
        let err = enumerate_err(p, &target);
        assert!(err.to_string().contains("contiguous stack"), "{err}");
    }

    /// A single commit still means the commit, not a one-long stack — the
    /// existing behavior must not regress.
    #[test]
    fn a_single_commit_still_diffs_against_its_parent() {
        let (dir, _base, tip) = stack_repo();
        let p = dir.path();

        let prepared = open(p)
            .enumerate(&DiffTarget::Revision { rev: tip.clone() }, &[])
            .unwrap();
        let paths: Vec<&str> = prepared
            .entries
            .iter()
            .map(|e| e.new_path.as_deref().unwrap())
            .collect();
        assert_eq!(paths, vec!["c.txt"], "only the tip commit's own change");
        assert!(prepared.label.unwrap().contains(&tip));
    }

    /// A revset alias resolving to a stack works exactly like the literal
    /// range — the point of the whole feature.
    #[test]
    fn a_revset_alias_naming_a_stack_is_reviewable() {
        let (dir, base, _tip) = stack_repo();
        let p = dir.path();
        // Emulate a user's `'ready()' = 'trunk()..@'` by aliasing the stack.
        fs::write(
            p.join(".jj/repo/config.toml"),
            format!("[revset-aliases]\n'ready()' = '{base}..tip'\n"),
        )
        .unwrap();

        let prepared = open(p)
            .enumerate(
                &DiffTarget::Revision {
                    rev: "ready()".into(),
                },
                &[],
            )
            .unwrap();
        let paths: Vec<&str> = prepared
            .entries
            .iter()
            .map(|e| e.new_path.as_deref().unwrap())
            .collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "c.txt"]);
    }

    /// The label leads with the change id: it survives rewrites, so an agent
    /// that quotes it back can still act on it after an amend.
    #[test]
    fn label_carries_the_change_id() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        let change = jj(
            dir.path(),
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short(8)"],
        );

        let prepared = open(dir.path())
            .enumerate(&DiffTarget::WorkingCopy, &[])
            .unwrap();
        assert_eq!(prepared.label.unwrap(), format!("@ · {change}"));
    }

    /// Zero and many are errors, never a silent "take the first" — a divergent
    /// change id makes "many" a real case, not a corner.
    #[test]
    fn revset_must_resolve_to_exactly_one_commit() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        jj(dir.path(), &["describe", "-m", "base"]);
        jj(dir.path(), &["new"]);

        let none = enumerate_err(
            dir.path(),
            &DiffTarget::Revision {
                rev: "none()".into(),
            },
        );
        assert!(none.to_string().contains("didn't resolve"), "{none}");

        let bogus = enumerate_err(
            dir.path(),
            &DiffTarget::Revision {
                rev: "no-such-rev-zzz".into(),
            },
        );
        assert!(bogus.to_string().contains("no-such-rev-zzz"), "{bogus}");

        // A *range* endpoint has no stack interpretation to fall back on —
        // "the tree at these four commits" is meaningless — so there the
        // exactly-one rule still bites.
        let ambiguous = enumerate_err(
            dir.path(),
            &DiffTarget::Range {
                from: "all()".into(),
                to: AT.into(),
                merge_base: false,
            },
        );
        assert!(ambiguous.to_string().contains("resolved to"), "{ambiguous}");
        assert!(
            ambiguous.to_string().contains("name one of them"),
            "{ambiguous}"
        );
    }

    /// `all()` in a linear repo really *is* one contiguous stack (root → @), so
    /// it reviews as one — the whole history as additions. Surprising at a
    /// glance, but it is the honest answer, and the same thing
    /// `jj diff --from root() --to @` shows. Branch the repo and it goes back
    /// to being an error, because then it has two heads.
    #[test]
    fn all_is_a_stack_when_linear_and_an_error_when_branched() {
        let dir = jj_repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "alpha\n").unwrap();
        jj(p, &["describe", "-m", "base"]);
        let base = jj(
            p,
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        );
        jj(p, &["new"]);
        fs::write(p.join("b.txt"), "bravo\n").unwrap();
        jj(p, &["describe", "-m", "second"]);

        let linear = open(p)
            .enumerate(
                &DiffTarget::Revision {
                    rev: "all()".into(),
                },
                &[],
            )
            .unwrap();
        assert_eq!(
            linear
                .entries
                .iter()
                .map(|e| e.new_path.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["a.txt", "b.txt"]
        );

        // Fork a sibling off the base: now all() has two heads.
        jj(p, &["new", &base]);
        fs::write(p.join("c.txt"), "charlie\n").unwrap();
        jj(p, &["describe", "-m", "sibling"]);

        let err = enumerate_err(
            p,
            &DiffTarget::Revision {
                rev: "all()".into(),
            },
        );
        assert!(err.to_string().contains("contiguous stack"), "{err}");
    }

    /// A range with an empty side means "the current revision" — `@` here, not
    /// git's `HEAD`.
    #[test]
    fn empty_range_side_means_at() {
        let dir = jj_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        jj(dir.path(), &["describe", "-m", "base"]);
        jj(dir.path(), &["new"]);
        fs::write(dir.path().join("b.txt"), "bravo\n").unwrap();

        let prepared = open(dir.path())
            .enumerate(
                &DiffTarget::Range {
                    from: "@-".into(),
                    to: String::new(),
                    merge_base: false,
                },
                &[],
            )
            .unwrap();
        assert_eq!(prepared.label.as_deref(), Some("@-..@"));
        assert_eq!(find(&prepared.entries, "b.txt").status, FileStatus::Added);
    }

    /// jj has no index. Erroring beats inventing a meaning for `--staged`.
    #[test]
    fn staged_is_rejected_with_a_hint() {
        let dir = jj_repo();
        let err = enumerate_err(dir.path(), &DiffTarget::Staged);
        assert!(err.to_string().contains("no staging area"), "{err}");
        assert!(err.to_string().contains("working_copy"), "{err}");
    }

    /// In a colocated repo, git's HEAD and index are export artifacts of jj's
    /// state — so the jj tier wins, and `Staged` is still meaningless.
    #[test]
    fn colocated_repo_prefers_the_jj_tier() {
        let dir = jj_colocated_repo();
        fs::write(dir.path().join("a.txt"), "alpha\n").unwrap();
        assert!(dir.path().join(".git").exists());

        assert!(matches!(vcs::open(dir.path()).unwrap(), Backend::Jj(_)));

        let prepared = vcs::prepare(dir.path(), &DiffTarget::WorkingCopy, &[]).unwrap();
        assert_eq!(find(&prepared.entries, "a.txt").status, FileStatus::Added);

        let err = match vcs::prepare(dir.path(), &DiffTarget::Staged, &[]) {
            Err(e) => e,
            Ok(_) => panic!("Staged must be rejected in a jj repo"),
        };
        assert!(err.to_string().contains("no staging area"), "{err}");
    }

    /// jj carries conflicts around as committed objects — "what did this
    /// rebase break?" is exactly the review a user wants. So a conflicted file
    /// is content, not an error: it materializes to jj's own `%%%%%%%` marker
    /// text, the same bytes `jj diff` puts in front of the user.
    #[test]
    fn conflicted_files_render_as_marker_text() {
        let dir = jj_repo();
        let p = dir.path();
        fs::write(p.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        jj(p, &["describe", "-m", "base"]);
        let base = jj(
            p,
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        );

        // Two siblings edit the same line...
        jj(p, &["new"]);
        fs::write(p.join("a.txt"), "one\nSIDE A\nthree\n").unwrap();
        jj(p, &["describe", "-m", "a"]);
        jj(p, &["bookmark", "create", "-r", "@", "side-a"]);

        jj(p, &["new", &base]);
        fs::write(p.join("a.txt"), "one\nSIDE B\nthree\n").unwrap();
        jj(p, &["describe", "-m", "b"]);
        jj(p, &["bookmark", "create", "-r", "@", "side-b"]);

        // ...and a rebase stacks them: side-b now carries a committed conflict.
        // "What did this rebase break?" is exactly the review a jj user wants.
        jj(p, &["rebase", "-r", "side-b", "-d", "side-a"]);

        let target = DiffTarget::Revision {
            rev: "side-b".into(),
        };
        let entries = enumerate(p, &target);
        let conflicted = find(&entries, "a.txt");
        // The file is reviewable — not an error, not "unavailable".
        assert!(conflicted.new_oid.is_some());

        let content = text(p, &target, "a.txt", Side::New).expect("conflict must have text");
        assert!(
            content.contains("<<<<<<<") && content.contains("%%%%%%%"),
            "expected jj diff-style conflict markers, got:\n{content}"
        );
        assert!(
            content.contains("SIDE A") && content.contains("SIDE B"),
            "{content}"
        );
    }

    /// Binary content gates to `None` — the same capability signal the git
    /// tier gives, so the UI's "can unfold?" logic needs no jj special case.
    #[test]
    fn binary_content_is_unavailable_but_listed() {
        let dir = jj_repo();
        fs::write(dir.path().join("bin.dat"), b"\x00\x01binary").unwrap();

        let entries = enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert_eq!(find(&entries, "bin.dat").status, FileStatus::Added);
        assert!(text(dir.path(), &DiffTarget::WorkingCopy, "bin.dat", Side::New).is_none());
    }

    /// Filesets stand in for pathspecs — `src/` is a bare path in jj's dialect.
    #[test]
    fn filesets_limit_the_diff() {
        let dir = jj_repo();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/a.txt"), "alpha\n").unwrap();
        fs::write(dir.path().join("b.txt"), "bravo\n").unwrap();

        let entries = open(dir.path())
            .enumerate(&DiffTarget::WorkingCopy, &["src".to_string()])
            .unwrap()
            .entries;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].new_path.as_deref(), Some("src/a.txt"));
    }

    /// jj's own ignore rules (via `snapshot.auto-track` and .gitignore) decide
    /// what's in `@` — we honor them by snapshotting through jj-lib rather than
    /// reimplementing tracking.
    #[test]
    fn gitignored_files_do_not_appear() {
        let dir = jj_repo();
        fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(dir.path().join("ignored.txt"), "invisible\n").unwrap();

        let entries = enumerate(dir.path(), &DiffTarget::WorkingCopy);
        assert!(
            entries
                .iter()
                .all(|e| e.new_path.as_deref() != Some("ignored.txt")),
            "{entries:?}"
        );
    }
}
