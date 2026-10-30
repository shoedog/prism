import os
from pathlib import Path
import subprocess
import sys


def test_inherited_private_cache_marker_cannot_authorize_child_import(tmp_path):
    eval_dir = Path(__file__).parents[1]
    package = tmp_path / "tier_a"
    package.mkdir()
    (package / "__init__.py").write_text("")
    source = package / "cli.py"
    stale = 'def main():\n    print("stale")\n    return 0\n'
    fresh = 'def main():\n    print("fresh")\n    return 0\n'
    source.write_text(stale)
    fixed_time = 1_700_000_000
    os.utime(source, (fixed_time, fixed_time))
    inherited_cache = tmp_path / "tier-a-pycache-inherited"
    inherited_cache.mkdir()
    compile_env = dict(os.environ)
    compile_env["PYTHONPYCACHEPREFIX"] = str(inherited_cache)
    subprocess.run(
        [sys.executable, "-m", "py_compile", str(source)],
        check=True,
        env=compile_env,
    )
    source.write_text(fresh)
    os.utime(source, (fixed_time, fixed_time))
    env = dict(compile_env)
    env["PYTHONPATH"] = os.pathsep.join((str(tmp_path), str(eval_dir)))
    env["TIER_A_PRIVATE_PYCACHE"] = str(inherited_cache)

    result = subprocess.run(
        [sys.executable, "-c", "import tier_a_entrypoint; tier_a_entrypoint.main()"],
        check=True,
        capture_output=True,
        text=True,
        env=env,
        cwd=tmp_path,
    )

    assert result.stdout.strip() == "fresh"
