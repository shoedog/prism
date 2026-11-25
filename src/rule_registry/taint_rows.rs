//! Public taint identities from the lane 7 design, Appendices A, B and F.

use super::RuleInfo;

macro_rules! taint_rows {
    ($($constant:ident, $id:literal, $cwe:expr, $short:literal, $help:literal;)*) => {
        $(
            pub const $constant: RuleInfo = RuleInfo {
                id: $id,
                cwe: $cwe,
                name: $id,
                short: $short,
                help: $help,
                algorithms: &["taint"],
            };
        )*
        pub(super) static RULES: &[RuleInfo] = &[$($constant),*];

        #[cfg(test)]
        #[test]
        fn named_rows_match_lookup_by_value() {
            $(assert_eq!(super::lookup($constant.id()), Some(&$constant));)*
        }
    };
}

taint_rows! {
    BROWSER_OPEN, "browser-open", None, "A call to open a browsing target receives a tainted destination.", "Check how the target is opened and constrain its scheme and destination to the intended set.";
    CONFIGURED_SINK, "configured-sink", None, "A configured sink name matches a call with a tainted argument.", "Inspect the actual callee and determine how it uses the reached argument.";
    EXTERNAL_OPEN, "external-open", None, "An external opener hands a tainted target to the operating system.", "Identify the default handler and validate the target before handing it off.";
    FORMAT_WRAPPER, "format-wrapper", None, "A detected variadic wrapper receives a tainted argument.", "Trace the argument through the wrapper to determine whether it supplies a format or a substitution.";
    GO_DB_CALL, "go-db-call", Some("CWE-89"), "A database-like Go call receives a tainted statement argument.", "Confirm the receiver and statement slot, then bind untrusted values as parameters.";
    GO_EXEC_COMMAND, "go-exec-command", Some("CWE-78"), "A Go process launcher receives a tainted executable or shell command.", "Check which slot selects the program or command and constrain it to trusted values.";
    GO_FILE_MODE, "go-file-mode", None, "A file mode operation receives a tainted permission mask.", "Check the effective permissions of the open file against its access policy.";
    GO_FILE_OP, "go-file-op", Some("CWE-22"), "A filesystem-like Go call receives a tainted path argument.", "Check every affected path against the intended directory boundary.";
    GO_FORMAT_STRING, "go-format-string", Some("CWE-134"), "A Go formatter receives tainted format text.", "Keep the format string fixed and pass untrusted data as substitutions.";
    GO_HTML_OUTPUT, "go-html-output", Some("CWE-79"), "A Go response writes tainted HTML content.", "Check the HTML context of the reached value and apply context-appropriate escaping.";
    GO_HTML_RENDER, "go-html-render", None, "A Gin HTML render receives a tainted template selector or context value.", "Check the template allowlist and whether the selected template escapes the reached value.";
    GO_REDIRECT, "go-redirect", Some("CWE-601"), "A Go redirect receives a tainted destination URL.", "Restrict the destination to trusted origins or validated local paths.";
    GO_TEMPLATE_HTML, "go-template-html", Some("CWE-79"), "A Go template trust conversion receives tainted HTML.", "Verify that the input is safe for HTML before marking it trusted.";
    GO_WRITER_OUTPUT, "go-writer-output", None, "A Go writer receives tainted output data.", "Identify the writer and downstream content type to determine how the data is interpreted.";
    JS_CHILD_MODULE, "js-child-module", Some("CWE-73"), "A Node child process receives a tainted module filename.", "Restrict the child module to known application paths.";
    JS_CHILD_PROCESS, "js-child-process", Some("CWE-78"), "A JavaScript process launcher receives a tainted program or shell-interpreted argument.", "Check the launcher and shell option, and keep untrusted data separate from executable selection and shell syntax.";
    JS_DB_CALL, "js-db-call", Some("CWE-89"), "A database-like JavaScript call receives a tainted statement or clause argument.", "Confirm the database receiver and statement slot, then use parameters for untrusted values.";
    JS_DOM_HTML, "js-dom-html", Some("CWE-79"), "A DOM-style HTML assignment or call, or a JSX HTML attribute, receives tainted markup.", "Confirm the HTML target and use a text-only API for data or sanitize intentional markup.";
    JS_EVAL, "js-eval", Some("CWE-95"), "JavaScript eval receives tainted source text.", "Trace the reached text and use a data parser if only data is expected.";
    JS_EXEC_CALL, "js-exec-call", Some("CWE-78"), "A JavaScript remote command call receives tainted command text.", "Keep the remote command fixed and verify how the API passes untrusted arguments.";
    JS_FS, "js-fs", Some("CWE-22"), "A filesystem-like JavaScript call receives a tainted path argument.", "Confirm the filesystem binding and path slots before checking that each path stays within the intended root.";
    JS_FUNCTION_CTOR, "js-function-ctor", Some("CWE-95"), "The JavaScript Function constructor receives a tainted body or parameter string.", "Check which string is parsed as code and replace dynamic construction with a fixed function.";
    JS_LOCATION, "js-location", Some("CWE-601"), "A navigation-like assignment or call, or a call matching a redirect-status and Location-header pattern, receives a tainted destination.", "Confirm the navigation or response binding, then check the destination and any redirect status.";
    JS_LOCATION_HEADER, "js-location-header", None, "A header-setting call shape matches a tainted Location value without an established redirect status.", "Confirm the response binding and check whether the status makes the header a redirect destination.";
    JS_REDIRECT, "js-redirect", Some("CWE-601"), "A redirect-like JavaScript call receives a tainted destination argument.", "Confirm the receiver redirects before checking the destination against trusted origins or local paths.";
    JS_RESPONSE, "js-response", Some("CWE-79"), "A response-like JavaScript call receives a tainted body.", "Check the actual media type and serialization, and escape HTML content when applicable.";
    JS_STORAGE_WRITE, "js-storage-write", Some("CWE-22"), "A storage-like JavaScript write call receives a tainted destination path.", "Confirm the storage binding and constrain the destination to the intended storage root.";
    JS_WRITE, "js-write", None, "A write-like JavaScript call receives a tainted payload.", "Identify the receiver and check whether its consumer interprets the payload as HTML.";
    LEGACY, "legacy", None, "A legacy sink name matched on a tainted line.", "Inspect the matched name and trace the input to the operation. This detector does not identify a particular operand or weakness.";
    PY_ATTRIBUTE_WRITE, "py-attribute-write", Some("CWE-915"), "A Python attribute write receives a tainted attribute name.", "Restrict the set of attributes that callers can modify.";
    PY_DB_CALL, "py-db-call", Some("CWE-89"), "A database-like Python call receives a tainted statement argument.", "Confirm the statement slot and pass untrusted values through the driver's parameter interface.";
    PY_DYNAMIC_ATTR, "py-dynamic-attr", None, "A Python attribute read receives a tainted attribute name.", "Inspect how the selected attribute is used and restrict access to intended names.";
    PY_EVAL, "py-eval", Some("CWE-95"), "A Python evaluation call receives tainted source text.", "Check the built-in binding and use a data parser when only data is required.";
    PY_FILE_OPEN, "py-file-open", Some("CWE-22"), "A Python file open receives a tainted path.", "Check the argument or the path captured by the receiver against the intended directory boundary.";
    PY_FILE_REMOVE, "py-file-remove", Some("CWE-22"), "A Python file removal receives a tainted path.", "Check the resolved target against the directory and files the caller may remove.";
    PY_HTML, "py-html", Some("CWE-79"), "An HTML trust helper match or an unescaped Python template output receives a tainted value.", "Confirm the helper binding or template use and keep escaping enabled for the output context.";
    PY_MODULE_COMMAND, "py-module-command", Some("CWE-78"), "A Python command-library call receives a tainted program or command.", "Check the imported API and separate untrusted data from program selection and command syntax.";
    PY_MODULE_TEMPLATE, "py-module-template", Some("CWE-1336"), "A Jinja environment receives tainted template source.", "Check whether the environment is sandboxed and keep untrusted data in the template context.";
    PY_OS_COMMAND, "py-os-command", Some("CWE-78"), "A Python OS process call receives a tainted program or command.", "Check the launcher signature and constrain the executable or shell text to trusted values.";
    PY_OUTPUT, "py-output", Some("CWE-79"), "A Python HTML response receives a tainted body.", "Inspect how the handler writes the body and escape the reached value for its HTML context.";
    PY_REDIRECT, "py-redirect", Some("CWE-601"), "A Python redirect receives a tainted destination.", "Verify the redirect binding and check the target against trusted origins or local paths.";
    PY_REFLECTIVE_DISPATCH, "py-reflective-dispatch", Some("CWE-470"), "A Python computed attribute name selects the callable to invoke.", "Restrict the selectable callables to an explicit allowlist.";
    PY_SQL_CLAUSE, "py-sql-clause", Some("CWE-89"), "A Python SQL clause or text constructor receives tainted SQL syntax.", "Keep clause syntax fixed and bind the reached value separately.";
    PY_SUBPROCESS, "py-subprocess", Some("CWE-78"), "A Python subprocess call receives tainted shell text or a tainted executable.", "Check the shell option and the executable slot, and pass data in separate arguments.";
    PY_TEMPLATE, "py-template", Some("CWE-1336"), "A Python template constructor or template-rendering call pattern receives tainted template source.", "Confirm the template API and source slot, then supply untrusted values through the template context.";
    STRUCTURED_CLIENT_REQUEST, "structured-client-request", None, "A browser-only request receives a tainted destination.", "Confirm the client-side binding and constrain destinations that can receive the request.";
    STRUCTURED_CODE_EVAL, "structured-code-eval", Some("CWE-95"), "A structured evaluation pattern receives tainted executable text.", "Confirm the code or predicate slot and avoid interpreting untrusted strings as code.";
    STRUCTURED_DESERIALIZATION, "structured-deserialization", Some("CWE-502"), "A structured deserialization call receives a tainted document or stream.", "Check the selected loader and restrict it to trusted types or a data-only format.";
    STRUCTURED_NAME_MATCH, "structured-name-match", None, "A structured sink name matches a tainted argument of an unresolved operation.", "Resolve the callee and inspect what it does with the indexed operand.";
    STRUCTURED_OS_COMMAND, "structured-os-command", Some("CWE-78"), "A structured process pattern receives a tainted executable or shell command.", "Verify the reached argument and shell mode, and keep command syntax and executable selection trusted.";
    STRUCTURED_PATH_TRAVERSAL, "structured-path-traversal", Some("CWE-22"), "A filesystem-like or file-serving call pattern receives a tainted path argument.", "Confirm the file API and check the indexed path against the intended root, including both paths for a move.";
    STRUCTURED_SQLI, "structured-sqli", Some("CWE-89"), "A structured database-like call pattern receives a tainted statement or SQL fragment.", "Confirm the callee and statement slot before checking how untrusted values are bound.";
    STRUCTURED_SSRF, "structured-ssrf", Some("CWE-918"), "A structured request call receives a tainted URL or destination option.", "Check whether the request can run server-side and restrict the destinations it can reach.";
    STRUCTURED_TEMPLATE, "structured-template", Some("CWE-1336"), "A structured template-rendering call pattern receives tainted template source.", "Confirm the renderer and source binding before passing untrusted values as context data.";
    STRUCTURED_XSS, "structured-xss", Some("CWE-79"), "A structured HTML pattern receives tainted markup, format text or unescaped context data.", "Confirm the HTML binding and check how the reached value is escaped in its output context.";
    TAINT_EVAL_NAME, "taint-eval-name", None, "An evaluation-like call receives a tainted argument.", "Resolve the callee to establish whether it evaluates the reached value as code.";
    TAINT_EXEC_NAME, "taint-exec-name", None, "An execution-like call receives a tainted argument.", "Resolve the callee and receiver to determine whether the argument selects a program or command.";
    TAINT_HTML_NAME, "taint-html-name", None, "An HTML-named helper receives a tainted argument.", "Inspect the helper to determine whether the operand is rendered as HTML and whether it is escaped.";
    TAINT_PROCESS_ARGUMENT, "taint-process-argument", None, "A process call receives tainted arguments, options or environment data.", "Inspect the invoked program and shell mode to determine whether the data becomes executable syntax.";
    TAINT_QUERY_NAME, "taint-query-name", None, "A query-like call receives a tainted prospective statement or clause.", "Resolve the receiver and identify whether the reached operand supplies query syntax or data.";
    TAINT_QUERY_VALUE, "taint-query-value", None, "A database-like or ORM call receives a tainted value as a bound value, a comparison operand or an opaque entity.", "Confirm the receiver, then determine which role the value plays (bound, compared, or selecting what is returned) and keep it bound or compared as data.";
    TAINT_REDIRECT_NAME, "taint-redirect-name", None, "A redirect-named call receives a tainted prospective destination.", "Resolve the callback before concluding that it redirects and then check its destination policy.";
    TAINT_REMOVE_NAME, "taint-remove-name", None, "A removal-like call receives a tainted argument.", "Resolve the receiver to determine whether the argument selects a collection item, asset or filesystem path.";
    TAINT_WRITE_DATA, "taint-write-data", None, "A write-like, send-like or response-like call receives tainted payload data.", "Resolve the callee and identify whether the downstream consumer interprets the data as active content.";
}
