use super::targets_inventory_constructors_test::{assert_exact, assert_refusal, source_document};

fn field(write: &str) -> String {
    format!(
        "from requests import Session\nclass API:\n    def init(self):\n        self.s = Session()\n    def alter(self, other):\n        {write}\n    def fetch(self, u):\n        return self.s.get(u)\n"
    )
}

#[test]
fn field_receiver_stores_and_lexical_owner() {
    for write in [
        "self.s = other",
        "self.s: object = other",
        "self.s += other",
        "self.s, *_ = other",
        "del self.s",
        "for self.s in other: pass",
        "[None for self.s in other]",
        "with other as self.s: pass",
        "setattr(self, 's', other)",
        "delattr(self, 's')",
        "self.__dict__['s'] = other",
        "vars(self)['s'] = other",
        "alias = self\n        alias.s = other",
        "def alter():\n            self.s = other",
    ] {
        assert_refusal(&source_document(&field(write)), "receiver_not_unique");
        assert_refusal(
            &source_document(&field("self.t = other")),
            "receiver_value_unproven",
        );
    }
    for write in [
        "self.s: object",
        "other.s = other",
        "setattr(self, computed, other)",
        "self.s.headers = other",
        "class API:\n            def alter(self, other):\n                self.s = other",
    ] {
        assert_refusal(&source_document(&field(write)), "receiver_value_unproven");
    }
    assert_exact(
        &source_document(
            "from requests import Session\ndef fetch(u):\n    s = Session()\n    return s.get(u)\n",
        ),
        "requests.Session.get",
    );
}

#[test]
fn field_receiver_selected_member_and_constructor() {
    for write in [
        "self.s.get = other",
        "alias = self.s\n        alias.get = other",
        "self.s.__dict__['get'] = other",
    ] {
        assert_refusal(&source_document(&field(write)), "member_rebound");
        assert_refusal(
            &source_document(&field(&write.replace("get", "post"))),
            "receiver_value_unproven",
        );
    }
    assert_refusal(
        &source_document(
            &field("pass").replace("class API:", "Session.__new__ = other\nclass API:"),
        ),
        "member_rebound",
    );
    assert_refusal(&source_document(&field("pass")), "receiver_value_unproven");
}
