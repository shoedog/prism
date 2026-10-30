"""One resolver for lock-declared artifacts, published or still staged."""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


def _relative_path(stored: str) -> Path:
    path = Path(stored)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"non-canonical lock path: {stored}")
    return path


@dataclass(frozen=True)
class ArtifactPaths:
    repo: Path
    prism: dict
    staged: dict[Path, Path] = field(default_factory=dict)

    @classmethod
    def from_lock(cls, repo: Path, lock) -> ArtifactPaths:
        return cls(repo.resolve(), lock.prism)

    @classmethod
    def from_candidate(cls, candidate) -> ArtifactPaths:
        staged = {
            **candidate.staged,
            **getattr(candidate, "_staged_archive_inputs", {}),
        }
        return cls(candidate._repo.resolve(), candidate.lock.prism, staged)

    def published(self, stored: str) -> Path:
        return self.repo / _relative_path(stored)

    def resolve(self, stored: str) -> Path:
        published = self.published(stored)
        return self.staged.get(published, published)

    def closure_manifest(self) -> Path:
        return self.resolve(self.prism["closure"]["manifest"])

    def sut_inputs(self) -> Path:
        return self.resolve(self.prism["sut"]["inputs"])

    def policy_manifest(self) -> Path:
        return self.resolve(self.prism["policy"]["manifest"])

    def population_manifest(self) -> Path:
        return self.resolve(self.prism["population"]["manifest"])

    def snapshot(self) -> Path:
        return self.resolve(self.prism["snapshot"]["path"])

    def sut_archive(self) -> Path:
        path = Path(self.prism["sut"]["archive"])
        if not path.is_absolute():
            raise ValueError(f"non-absolute SUT archive path: {path}")
        return path

    def sut_archive_input(self, relative: str) -> Path:
        final = self.sut_archive().parent / "inputs" / _relative_path(relative)
        return self.staged.get(final, final)

    def declared_open(self, path: Path) -> str:
        published = next(
            (final for final, staged in self.staged.items() if staged == path),
            path,
        )
        try:
            return published.relative_to(self.repo).as_posix()
        except ValueError:
            return str(published)
