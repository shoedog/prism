from tier_c.ensemble import EnsembleVerdict, parse_verdict, ensemble


def test_parse_verdict_yes_no_first():
    assert parse_verdict("YES, because line 42 is the buggy branch.", ("YES", "NO")) == \
        ("YES", "YES, because line 42 is the buggy branch.", False)
    assert parse_verdict("no - unrelated helper", ("YES", "NO"))[0] == "NO"


def test_parse_verdict_abtie():
    assert parse_verdict("TIE — both miss the root cause", ("A", "B", "TIE"))[0] == "TIE"
    assert parse_verdict("B is better", ("A", "B", "TIE"))[0] == "B"


def test_parse_verdict_unparsed_flagged():
    v, reason, unparsed = parse_verdict("I think probably yes", ("YES", "NO"))
    assert v == "" and unparsed is True and reason == "I think probably yes"


def test_ensemble_agree_no_opus():
    calls = []
    def ask(model, prompt):
        calls.append(model)
        return "YES, relevant"
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.verdict == "YES" and ev.escalated is False
    assert calls == ["sonnet-4.6", "sonnet-4.6"]
    assert len(ev.votes) == 2


def test_ensemble_disagree_escalates_to_opus():
    seq = iter(["YES because X", "NO because Y"])
    calls = []
    def ask(model, prompt):
        calls.append(model)
        return "NO, opus says unrelated" if model == "opus-4.8" else next(seq)
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.escalated is True
    assert ev.verdict == "NO"
    assert calls == ["sonnet-4.6", "sonnet-4.6", "opus-4.8"]
    assert len(ev.votes) == 3 and ev.votes[-1]["model"] == "opus-4.8"


def test_ensemble_unparsed_uses_default():
    def ask(model, prompt):
        return "hmm, hard to say"
    ev = ensemble(ask, "p", ("YES", "NO"), sonnet="sonnet-4.6", opus="opus-4.8", default="NO")
    assert ev.verdict == "NO" and ev.escalated is False
    assert all(v["unparsed"] for v in ev.votes)
