use super::{family, Family, STDLIB};
use std::collections::BTreeMap;

#[test]
fn stdlib_table_matches_design_in_both_directions() {
    // DESIGN §5.3 canonical callees.
    let expected = [
        (
            Family::Filesystem,
            45,
            "builtins.open io.open pathlib.Path.open pathlib.Path.read_text pathlib.Path.read_bytes
             pathlib.Path.stat pathlib.Path.exists pathlib.Path.is_file pathlib.Path.is_dir
             pathlib.Path.write_text pathlib.Path.write_bytes pathlib.Path.mkdir pathlib.Path.touch
             pathlib.Path.rename pathlib.Path.replace pathlib.Path.unlink pathlib.Path.rmdir
             pathlib.Path.symlink_to os.stat os.listdir os.scandir os.pathconf os.mkdir os.makedirs
             os.remove os.unlink os.rename os.replace os.rmdir os.write os.fsync os.symlink os.link
             os.path.getsize os.path.getmtime shutil.copy shutil.copy2 shutil.copyfile
             shutil.copytree shutil.move tempfile.mkstemp tempfile.mkdtemp tempfile.NamedTemporaryFile
             tempfile.TemporaryFile tempfile.TemporaryDirectory",
        ),
        (
            Family::Parse,
            20,
            "json.load json.loads tomllib.load tomllib.loads tomli.load tomli.loads
             configparser.ConfigParser.read configparser.ConfigParser.read_file
             configparser.ConfigParser.read_string configparser.RawConfigParser.read
             configparser.RawConfigParser.read_file configparser.RawConfigParser.read_string
             xml.etree.ElementTree.parse xml.etree.ElementTree.fromstring xml.dom.minidom.parse
             xml.dom.minidom.parseString defusedxml.minidom.parse defusedxml.minidom.parseString
             pickle.load pickle.loads",
        ),
        (
            Family::Config,
            15,
            "os.environ.get os.getenv importlib.metadata.version importlib.metadata.metadata
             importlib.metadata.distribution importlib.metadata.files importlib.metadata.entry_points
             importlib_metadata.version importlib_metadata.metadata importlib_metadata.distribution
             importlib_metadata.files importlib_metadata.entry_points pathlib.Path.home
             locale.getpreferredencoding locale.getencoding",
        ),
    ];
    let mut design = BTreeMap::new();
    for (kind, count, paths) in expected {
        assert_eq!(paths.split_whitespace().count(), count);
        for path in paths.split_whitespace() {
            assert!(design.insert(path.to_owned(), kind).is_none(), "{path}");
            assert_eq!(family(path), Some(kind), "{path}");
        }
    }
    let mut table = BTreeMap::new();
    for &(owner, kind, members) in STDLIB {
        for member in members {
            let path = format!("{owner}.{member}");
            assert!(table.insert(path.clone(), kind).is_none(), "{path}");
        }
    }
    assert_eq!(table, design);
}
