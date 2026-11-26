from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
from unittest.mock import patch

from .helpers import MEASURE, Workbench, fixture_digest, write_tree


class CorporaTests(Workbench):
    def test_corpora_verify_nonzero_on_mismatch(self):
        self.commit_files({"other.ts": "export {};\n"})
        result = self.verify()
        self.assert_exit(result, 3)
        self.assertIn("Z\trefused\t", result.stdout)

    def test_manifest_hash_matches_perf_cloud(self):
        from measure.inputs import manifest_hash
        from perf.cloud.fetch_corpora import manifest_hash as perf_hash

        self.fresh_tree(files={"a.ts": "a", "sub/b.ts": "b", "a\\b": "c", "a\nb": "d"},
                        links={"alias.ts": "a.ts"})
        expected = perf_hash(self.source)
        self.assertEqual(manifest_hash(self.source), expected)
        self.entry["manifest_sha256"] = expected
        self.write_manifest()
        self.assert_exit(self.verify(), 0)
        self.entry["manifest_sha256"] = "0" * 64
        self.write_manifest()
        self.assert_exit(self.verify(), 3)

    def test_tree_pin_detects_link_changes_with_same_perf_manifest(self):
        from measure.inputs import manifest_hash

        self.fresh_tree(files={"a.ts": "a", "b.ts": "b"}, links={"alias": "a.ts"})
        self.entry["manifest_sha256"] = manifest_hash(self.source)
        self.write_manifest()
        self.assert_exit(self.verify(), 0)
        alias = self.source / "alias"
        for change in ("retarget", "remove", "add"):
            with self.subTest(change=change):
                alias.unlink()
                if change == "retarget":
                    alias.symlink_to("b.ts")
                if change == "add":
                    alias.symlink_to("a.ts")
                    (self.source / "extra-alias").symlink_to("a.ts")
                self.assertEqual(manifest_hash(self.source), self.entry["manifest_sha256"])
                self.assert_exit(self.verify(), 3)
                self.assert_exit(self.run_kit(), 3)
                self.assertEqual(self.fake_log(), [])
                alias.unlink(missing_ok=True)
                alias.symlink_to("a.ts")

    def test_registry_has_adopter_pins_and_rs_alias(self):
        registry = json.loads((MEASURE / "corpora.json").read_bytes())["corpora"]
        self.assertEqual(registry["RS"], registry["R"])
        self.assertEqual(registry["T"]["root"], "src")
        self.assertEqual(registry["EX"]["commit"], "18e5985b8a9d5e8423db0a9121f22bdaecd5b120")
        self.assertSetEqual(set(registry), set(
            "X EX T P1 P2 G1 G2 R RS G GIN D M GO PY C leveldb click httpx clap "
            "apache-answer calcom-cal.com directus-directus miniflux-v2".split()))

    def test_sb_registrations_bind_individual_package_roots(self):
        entries = {}
        for key, name in (("SB.fixtures.first", "first"), ("SB.fixtures.second", "second")):
            relative = f"fixtures/{name}/src/package"
            package = self.work / relative
            write_tree(package, {"index.js": name})
            entries[key] = dict(self.entry, name=key, kind="tree", commit=None, root=".",
                                local=[relative, f"pkgs/fixtures/{name}"], history=False,
                                cloud_asset="secbench-recall-v1", tree_sha256=fixture_digest(package))
        self.write_manifest(entries)
        self.assert_exit(self.verify(), 0)
        self.key = "SB.fixtures.first"
        self.assert_exit(self.run_kit(), 0)
        snapshot = Path(self.receipt()["corpus"]["root"])
        self.assertEqual((snapshot / "index.js").read_text(), "first")
        entries[self.key]["local"] = entries["SB.fixtures.second"]["local"]
        self.write_manifest(entries)
        self.assert_exit(self.verify(self.key), 2)

    def test_list_and_first_existing_candidate(self):
        self.assertIn("Z\tFixture\tgit\t" + self.commit, self.kit("corpora", "list").stdout)
        fallback = self.work / "fallback"
        shutil.copytree(self.source, fallback / self.source.name)
        self.env["PRISM_MEASURE_CORPORA"] = str(fallback)
        write_tree(self.source, {"extra.ts": "extra"})
        self.assert_exit(self.verify("Z"), 3)
        self.assert_exit(self.kit("corpora", "verify", "Z"), 0)
        self.assert_exit(self.verify("unknown"), 2)
        self.write_manifest({"Z": self.entry, "other": dict(self.entry, local=["missing"])})
        result = self.verify()
        self.assert_exit(result, 3)
        self.assertIn("Z\trefused\t", result.stdout)
        self.assertIn("other\trefused\t", result.stdout)

    def test_strict_manifest(self):
        valid = self.manifest.read_text(encoding="utf-8")
        for contents in (valid.replace('"schema":', '"schema":null,"schema":', 1),
                         valid.replace('"history": false', '"history": 0'),
                         valid.replace('"history": false', '"history": NaN'),
                         "\ufeff" + valid, valid + "{}"):
            with self.subTest(contents=contents[:70]):
                self.manifest.write_text(contents, encoding="utf-8")
                self.assert_exit(self.verify(), 2)

    def test_git_blob_binding_ignores_index_flags(self):
        for flag in ("assume-unchanged", "skip-worktree"):
            with self.subTest(flag=flag):
                self.git("update-index", "--" + flag, "a.ts")
                write_tree(self.source, {"a.ts": "hidden edit"})
                self.assert_exit(self.verify(), 3)
                self.assert_exit(self.run_kit(), 3)
                self.assertEqual(self.fake_log(), [])
                write_tree(self.source, {"a.ts": "export {};\n"})
                self.assert_exit(self.verify(), 0)
                self.git("update-index", "--no-" + flag, "a.ts")

    def test_link_hops_refuse_even_when_dangling(self):
        self.fresh_tree(links={"alias": "missing"})
        self.assert_exit(self.verify(), 0)
        for target in ("../shared/.env", ".git/missing", "/nonexistent",
                       "missing/../../shared/.env", "missing/.git/file"):
            with self.subTest(target=target):
                link = self.source / "alias"
                link.unlink()
                link.symlink_to(target)
                self.entry["tree_sha256"] = fixture_digest(self.source)
                self.write_manifest()
                result = self.verify()
                self.assert_exit(result, 3)
                self.assertIn("unbound-link", result.stdout)
                self.assert_exit(self.run_kit(), 3)
                self.assertEqual(self.fake_log(), [])

    def test_link_cycles_and_ancestor_directories_refuse(self):
        self.fresh_tree(files={"sub/file": "a"}, links={"a": "b", "b": "a"})
        self.assert_exit(self.verify(), 3)
        (self.source / "a").unlink()
        (self.source / "b").unlink()
        for name, target in (("up", "."), ("sub/up", "..")):
            with self.subTest(name=name):
                link = self.source / name
                link.symlink_to(target)
                self.entry["tree_sha256"] = fixture_digest(self.source)
                self.write_manifest()
                result = self.verify()
                self.assert_exit(result, 3)
                self.assertIn("unbound-link", result.stdout)
                link.unlink()

    def test_parent_after_link_uses_filesystem_resolution(self):
        self.fresh_tree(files={"sub/nested/file": "a", "sub/one": "b"}, links={
            "b": "sub/nested", "x": "b/../one",
        })
        self.assert_exit(self.verify(), 0)
        (self.source / "x").unlink()
        (self.source / "x").symlink_to("b/../../../outside")
        self.entry["tree_sha256"] = fixture_digest(self.source)
        self.write_manifest()
        self.assert_exit(self.verify(), 3)

    def test_link_state_binds_case_alias_resolution(self):
        from measure.inputs import bind_tree

        self.fresh_tree(files={"pkg/brew/input": "a"}, links={"alias": "Pkg/brew"})
        actual_stat = os.stat

        def stat_on_host(case_sensitive, path, *args, **kwargs):
            if path == b"Pkg" and "dir_fd" in kwargs:
                if case_sensitive:
                    raise FileNotFoundError("case-sensitive host")
                path = b"pkg"
            return actual_stat(path, *args, **kwargs)

        states = []
        for case_sensitive in (True, False):
            with patch("measure.inputs.os.stat", side_effect=lambda *args, **kwargs:
                       stat_on_host(case_sensitive, *args, **kwargs)):
                states.append(bind_tree(self.source))
        self.assertEqual(states[0].sha256, states[1].sha256)
        self.assertNotEqual(states[0].state_sha256, states[1].state_sha256)

    def test_root_link_and_parent_escape_refuse(self):
        original = self.source
        self.fresh_tree()
        (self.source / "src").symlink_to(original, target_is_directory=True)
        for root in ("src", "../corpus"):
            with self.subTest(root=root):
                self.entry.update(root=root, tree_sha256=fixture_digest(original))
                self.write_manifest()
                self.assert_exit(self.verify(), 3)
                self.assert_exit(self.run_kit(), 3)
                self.assertEqual(self.fake_log(), [])

    def test_gitlinks_refuse(self):
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.commit},submodule")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit",
                 "-qm", "gitlink")
        self.entry["commit"] = self.git("rev-parse", "HEAD")
        self.write_manifest()
        for populated in (False, True):
            with self.subTest(populated=populated):
                if populated:
                    write_tree(self.source, {"submodule/content": "unbound"})
                result = self.verify()
                self.assert_exit(result, 3)
                self.assertIn("gitlink", result.stdout)
                self.assert_exit(self.run_kit(), 3)

    def test_unreadable_or_special_member_refuses(self):
        self.fresh_tree()
        path = self.source / "a.ts"
        path.chmod(0)
        try:
            self.assert_exit(self.verify(), 3)
            self.assert_exit(self.run_kit(), 3)
        finally:
            path.chmod(0o644)
        os.mkfifo(self.source / "fifo")
        self.assert_exit(self.verify(), 3)
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])

    def test_hardlinks_refuse(self):
        self.fresh_tree()
        os.link(self.source / "a.ts", self.source / "also.ts")
        self.entry["tree_sha256"] = fixture_digest(self.source)
        self.write_manifest()
        self.assert_exit(self.verify(), 3)
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])

    def test_filter_converted_git_checkout_refuses_and_raw_bytes_admit(self):
        self.entry["commit"] = self.commit_files({".gitattributes": "*.ts text eol=crlf\n"})
        self.write_manifest()
        path = self.source / "a.ts"
        path.unlink()
        self.git("checkout", "--", "a.ts")
        self.assertIn(b"\r\n", path.read_bytes())
        self.assert_exit(self.verify(), 3)
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])
        path.write_bytes(b"export {};\n")
        # Refresh the unchanged index entry's stat before asserting clean status.
        self.git("add", "--", "a.ts")
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(self.git("rev-parse", "HEAD"), self.entry["commit"])
        self.assert_exit(self.run_kit(), 0)
        self.assertEqual((Path(self.receipt()["corpus"]["root"]) / "a.ts").read_bytes(),
                         b"export {};\n")

    def test_git_and_tree_pins_must_agree(self):
        self.entry["tree_sha256"] = "0" * 64
        self.write_manifest()
        self.assert_exit(self.verify(), 3)
        self.assert_exit(self.run_kit(), 3)
        self.assertEqual(self.fake_log(), [])
        self.entry["tree_sha256"] = fixture_digest(self.source)
        self.write_manifest()
        self.assert_exit(self.verify(), 0)
