# eval/adoption/tests/unit/test_twobytwo_skillname.py
from adoption.model import Trajectory
from adoption.twobytwo import loaded_skill_name

def test_loaded_skill_name():
    assert loaded_skill_name(Trajectory("a", ["prism-code-navigation"], [])) == "prism-code-navigation"
    assert loaded_skill_name(Trajectory("a", ["/x/prism-nav/SKILL.md"], [])) == "prism-nav"
    assert loaded_skill_name(Trajectory("a", [], [])) is None
