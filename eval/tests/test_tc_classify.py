from tier_c.classify import classify_tools

def test_classify_flags_lsp_and_compiler():
    assert classify_tools(["grep -n Foo src", "cat a.py"]) == {"lsp_leak": False, "compiler_assisted": False}
    assert classify_tools(["pyright a.py"])["lsp_leak"] is True
    assert classify_tools(["cargo check"])["compiler_assisted"] is True
    assert classify_tools(["go vet ./..."])["compiler_assisted"] is True

def test_launchers_are_not_lsp_leaks():
    # normal JS project setup must NOT flag as lsp_leak
    assert classify_tools(["yarn add foo"])["lsp_leak"] is False
    assert classify_tools(["pnpm install"])["lsp_leak"] is False
    assert classify_tools(["npx tsc"])["lsp_leak"] is True   # tsc inside npx still flags (tsc in command)
