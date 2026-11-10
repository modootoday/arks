//! Stock `git` in the clean-room image is the compatibility oracle (no gix).
//! Gate G4: pack/idx local round-trip vs stock Git (claimed M4 subset).
//!
//! Primary: https://git-scm.com/docs/gitformat-pack
//! Cite: `.spec/sot/arks-pack.sot.md`, `verify/matrices/matrix-m004-clean-room-pack-protocol.md`

use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject};
use arks_oid::{Oid, OidKind};
use arks_pack::{DecodedPack, EncodedPack, PackIndex};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn require_git() {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .expect("stock git must exist in clean-room image");
    assert!(
        out.status.success(),
        "git --version failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_init(path: &Path) {
    require_git();
    let out = Command::new("git")
        .args(["-c", "init.templateDir=", "init", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_hash_object_write(repo: &Path, kind: &str, payload: &[u8]) -> String {
    let mut child = Command::new("git")
        .args([
            "--git-dir",
            &repo.join(".git").to_string_lossy(),
            "hash-object",
            "-w",
            "-t",
            kind,
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(payload).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git hash-object: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn sample_objects() -> Vec<RawObject> {
    vec![
        RawObject::new(ObjectKind::Blob, b"".to_vec()),
        RawObject::new(ObjectKind::Blob, b"hello-g004-pack-idx-roundtrip".to_vec()),
        RawObject::new(ObjectKind::Blob, b"pack-local-roundtrip\n".to_vec()),
        RawObject::new(ObjectKind::Tree, b"".to_vec()),
    ]
}

fn find_idx_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut idxs: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("idx"))
        .collect();
    idxs.sort();
    idxs.pop()
}

fn find_idx_beside(pack_path: &Path) -> Option<PathBuf> {
    let parent = pack_path.parent()?;
    let stem = pack_path.file_stem()?.to_string_lossy();
    let direct = parent.join(format!("{stem}.idx"));
    if direct.is_file() {
        return Some(direct);
    }
    find_idx_in_dir(parent)
}

#[test]
fn arks_undeltified_pack_accepted_by_git_index_pack() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let objects = sample_objects();
    let encoded = EncodedPack::from_undeltified(ObjectFormat::Sha1, &objects).unwrap();

    let pack_path = tmp.path().join("arks.pack");
    fs::write(&pack_path, &encoded.pack).unwrap();

    let out = Command::new("git")
        .args(["index-pack", "--strict", pack_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git index-pack --strict: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let idx_from_git = find_idx_beside(&pack_path).expect("git index-pack must write an idx");
    let git_idx_bytes = fs::read(&idx_from_git).unwrap();
    let arks_idx = PackIndex::decode(&encoded.idx, ObjectFormat::Sha1).unwrap();
    let git_idx = PackIndex::decode(&git_idx_bytes, ObjectFormat::Sha1).unwrap();
    assert_eq!(arks_idx.entries.len(), git_idx.entries.len());
    for entry in &arks_idx.entries {
        let got = git_idx.lookup(&entry.oid).expect("oid present in git idx");
        assert_eq!(got.offset, entry.offset);
        assert_eq!(got.crc32, entry.crc32);
    }

    let verify = Command::new("git")
        .args(["verify-pack", "-v", pack_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "git verify-pack: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn git_pack_objects_decoded_by_arks() {
    require_git();
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git_init(&repo);

    let payloads: &[(&[u8], &str)] = &[
        (b"alpha-blob", "blob"),
        (b"beta-blob-payload", "blob"),
        (b"", "blob"),
    ];
    let mut oids = Vec::new();
    for (payload, kind) in payloads {
        oids.push(git_hash_object_write(&repo, kind, payload));
    }

    let pack_dir = tmp.path().join("packs");
    fs::create_dir_all(&pack_dir).unwrap();
    let pack_prefix = pack_dir.join("test");

    let mut child = Command::new("git")
        .args([
            "--git-dir",
            &repo.join(".git").to_string_lossy(),
            "pack-objects",
            pack_prefix.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for oid in &oids {
            writeln!(stdin, "{oid}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git pack-objects: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pack_sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let pack_path = pack_dir.join(format!("test-{pack_sha}.pack"));
    let pack_bytes = fs::read(&pack_path).expect("pack file from git pack-objects");

    let decoded = DecodedPack::decode(&pack_bytes, ObjectFormat::Sha1).unwrap();
    assert_eq!(decoded.objects.len(), oids.len());
    let mut got: Vec<String> = decoded.objects.iter().map(|o| o.oid.to_hex()).collect();
    got.sort();
    let mut want = oids.clone();
    want.sort();
    assert_eq!(got, want);

    for oid_hex in &oids {
        let oid = Oid::from_hex(oid_hex).unwrap();
        let resolved = decoded.find(&oid).expect("oid in decoded pack");
        assert_eq!(resolved.oid.kind(), OidKind::Sha1);
    }

    let idx_path = find_idx_in_dir(&pack_dir).expect("git pack-objects writes idx");
    let idx = PackIndex::decode(&fs::read(&idx_path).unwrap(), ObjectFormat::Sha1).unwrap();
    for oid_hex in &oids {
        let oid = Oid::from_hex(oid_hex).unwrap();
        assert!(idx.lookup(&oid).is_some());
    }
}

#[test]
fn arks_roundtrip_matches_own_oids() {
    let objects = sample_objects();
    let encoded = EncodedPack::from_undeltified(ObjectFormat::Sha1, &objects).unwrap();
    let decoded = DecodedPack::decode(&encoded.pack, ObjectFormat::Sha1).unwrap();
    assert_eq!(decoded.objects.len(), objects.len());
    for obj in &objects {
        let oid = obj.oid(ObjectFormat::Sha1).unwrap();
        let got = decoded.find(&oid).unwrap();
        assert_eq!(&got.object, obj);
    }
    let idx = PackIndex::decode(&encoded.idx, ObjectFormat::Sha1).unwrap();
    for obj in &objects {
        let oid = obj.oid(ObjectFormat::Sha1).unwrap();
        assert!(idx.lookup(&oid).is_some());
    }
}
