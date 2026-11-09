//! Refusing a transfer before anything is written.

mod support;

use std::path::Path;
use std::sync::Arc;

use offloader_core::{CopyMode, Job, JobSpec, PrepareError};
use support::*;
use tempfile::tempdir;

#[test]
fn not_enough_space_is_refused_before_writing_anything() {
    struct Tiny;
    impl offloader_core::Faults for Tiny {
        fn available_space(&self, _dest: &Path) -> Option<u64> {
            Some(10 << 20)
        }
    }
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let err = Job::prepare(JobSpec { faults: Some(Arc::new(Tiny)), ..spec(&card.root, &dest) }).err().unwrap();
    assert!(matches!(err, PrepareError::InsufficientSpace { .. }), "{err}");
    assert!(!dest.exists());
}

#[test]
fn refuses_to_write_inside_the_source() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let err = Job::prepare(spec(&card.root, &card.root.join("DCIM/backup"))).err().unwrap();
    assert!(matches!(err, PrepareError::DestinationInsideSource { .. }), "{err}");
    let err = Job::prepare(spec(&card.root, &card.root)).err().unwrap();
    assert!(matches!(err, PrepareError::DestinationInsideSource { .. }), "{err}");
}

#[test]
fn refuses_overlapping_destinations() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let d = tmp.path().join("dest");
    let s = spec_with_backup(&card.root, &d, &d.join("backup"), CopyMode::Sequential, false);
    assert!(matches!(Job::prepare(s).err().unwrap(), PrepareError::OverlappingDestinations { .. }));
    let s = spec_with_backup(&card.root, &d, &d, CopyMode::Sequential, false);
    assert!(matches!(Job::prepare(s).err().unwrap(), PrepareError::OverlappingDestinations { .. }));
}

#[test]
fn missing_source_is_reported() {
    let tmp = tempdir().unwrap();
    let err = Job::prepare(spec(&tmp.path().join("nope"), &tmp.path().join("dest"))).err().unwrap();
    assert!(matches!(err, PrepareError::SourceMissing(_)), "{err}");
}
