//! `offload plan`: shows what a copy would do, without writing anything.

use offloader_core::{Job, format};

pub fn print_plan(job: &Job, verbose: bool) {
    let plan = job.plan();
    let kind = match plan.source.kind {
        offloader_core::SourceKind::Volume => "drive",
        offloader_core::SourceKind::Folder => "folder",
    };
    eprintln!("Source   {} ({kind})  {}", plan.source.label, plan.source.root.display());
    eprintln!(
        "Files    {} · {}  ({} skipped, {} renamed, {} unreadable)",
        plan.files.len(),
        format::size(plan.total_bytes),
        plan.skipped.len(),
        plan.renamed.len(),
        plan.problems.len()
    );
    for d in job.destinations() {
        let role = match d.role {
            offloader_core::DestRole::Copy => "Copy",
            offloader_core::DestRole::Backup => "Backup",
        };
        eprintln!(
            "{role:<8} {}  ({}, {} free, needs {})",
            d.root.display(),
            if d.fs_type.is_empty() { "?" } else { &d.fs_type },
            format::size(d.available),
            format::size(d.required)
        );
        if d.conflict.is_some() {
            eprintln!(
                "         {} already holds files from this source; nothing there will be touched.",
                d.requested.display()
            );
        }
    }
    if verbose {
        for s in &plan.skipped {
            eprintln!("  skip     {}  ({:?})", format::rel_path(&s.rel), s.reason);
        }
        for r in &plan.renamed {
            eprintln!("  rename   {} → {}", format::rel_path(&r.from), format::rel_path(&r.to));
        }
    }
    for p in &plan.problems {
        eprintln!("  UNREADABLE  {}  ({})", format::rel_path(&p.rel), p.message);
    }
}
