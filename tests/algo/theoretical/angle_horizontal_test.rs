#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn make_error_handling_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
def fetch_data(url):
    try:
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        log.error(f"Failed to fetch {url}: {e}")
        raise

def process(url):
    try:
        data = fetch_data(url)
        return transform(data)
    except Exception:
        return None
"#;
    let path = "service.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]), // log.error line
        }],
    };

    (files, sources, diff)
}


#[test]
fn test_horizontal_slice_finds_peers() {
    let source = r#"
def handle_create(request):
    data = request.json()
    validate(data)
    return create_item(data)

def handle_update(request):
    data = request.json()
    return update_item(data)

def handle_delete(request):
    item_id = request.args.get("id")
    return delete_item(item_id)
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]), // validate(data) line in handle_create
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    // Should include peer functions (handle_update, handle_delete)
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("handlers.py").unwrap();
    assert!(
        lines.len() > 5,
        "HorizontalSlice should include peer functions, got {} lines",
        lines.len()
    );
}

#[test]
fn test_angle_slice_error_handling() {
    let (files, _, diff) = make_error_handling_test();
    let concern = prism::algorithms::angle_slice::Concern::ErrorHandling;
    let result = prism::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("service.py").unwrap();
    // Should find error handling patterns across both functions
    assert!(
        lines.len() > 3,
        "AngleSlice should trace error handling across functions"
    );
}

#[test]
fn test_angle_slice_python() {
    let source = r#"
import logging

def process(data):
    try:
        result = transform(data)
        logging.info("success")
        return result
    except Exception as e:
        logging.error(str(e))
        raise
"#;
    let path = "proc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9, 10]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_go() {
    let source = r#"package main

import "log"

func handler() error {
	result, err := doWork()
	if err != nil {
		log.Printf("error: %v", err)
		return err
	}
	log.Printf("success: %v", result)
	return nil
}

func doWork() (int, error) {
	return 42, nil
}
"#;
    let path = "handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_javascript_logging() {
    let source = r#"
function fetchData(url) {
    console.log("fetching", url);
    const res = fetch(url);
    if (res.error) {
        console.error("failed", res.error);
        return null;
    }
    console.log("done");
    return res;
}
"#;
    let path = "fetch.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Logging,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_custom_concern_python() {
    let source = r#"
import redis

def get_cached(key):
    cache = redis.get(key)
    if cache:
        return cache
    result = compute(key)
    redis.set(key, result, ttl=300)
    return result
"#;
    let path = "cache.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Caching,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_horizontal_slice_python() {
    let source = r#"
def handle_get(request):
    data = get_data()
    return data

def handle_post(request):
    data = request.body
    save_data(data)
    return "ok"

def handle_delete(request):
    delete_data(request.id)
    return "deleted"
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("handle_*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_javascript() {
    let source = r#"
function handleLogin(req, res) {
    const user = authenticate(req.body);
    res.send(user);
}

function handleLogout(req, res) {
    clearSession(req);
    res.send("ok");
}

function handleRegister(req, res) {
    const user = createUser(req.body);
    res.send(user);
}
"#;
    let path = "routes.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Auto,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_go() {
    let source = r#"package main

func HandleGet(w http.ResponseWriter, r *http.Request) {
	data := getData()
	w.Write(data)
}

func HandlePost(w http.ResponseWriter, r *http.Request) {
	body := r.Body
	saveData(body)
}
"#;
    let path = "routes.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("Handle*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_angle_slice_concern_not_on_diff_python() {
    // Concern exists in code but not on the diff lines themselves
    let source = r#"
def handler():
    try:
        result = compute()
    except Exception as e:
        log_error(e)
        raise
    return result

def compute():
    x = 1
    y = x + 1
    return y
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff is on lines 11-12 in compute(), which has no error handling patterns
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11, 12]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    // Should still find error handling code in the file
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_angle_slice_authentication_go() {
    let source = r#"package main

func authMiddleware(token string) bool {
	session := validateToken(token)
	if session == nil {
		return false
	}
	return authorize(session)
}

func validateToken(t string) interface{} {
	return nil
}

func authorize(s interface{}) bool {
	return true
}
"#;
    let path = "auth.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Authentication,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_horizontal_slice_name_suffix_python() {
    // Test NamePattern with suffix matching (*_handler)
    let source = r#"
def get_handler(request):
    return get_data()

def post_handler(request):
    save_data(request.body)

def delete_handler(request):
    remove_data(request.id)
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("*_handler".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_horizontal_slice_decorator_python() {
    // Test Decorator matching
    let source = r#"
@app.route("/users")
def get_users():
    return users

@app.route("/items")
def get_items():
    return items
"#;
    let path = "routes.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Decorator("@app.route".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}
