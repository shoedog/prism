use std::fs;
use std::path::{Path, PathBuf};

pub const ROWS: &str = include_str!("../fixtures/ids_matrix/rows.tsv");

pub const P1B_ROWS: &str = include_str!("../fixtures/p1b_rows/rows.tsv");

pub struct Row<'a> {
    pub id: &'a str,
    pub fixture: &'a str,
    pub operations: Vec<&'a str>,
    pub reason: Option<&'a str>,
    pub fields: Vec<&'a str>,
    pub warnings: Vec<&'a str>,
    pub class: &'a str,
    matrix: &'a str,
}

pub fn rows() -> impl Iterator<Item = Row<'static>> {
    matrix_rows("ids_matrix", ROWS)
}

pub fn p1b_rows() -> impl Iterator<Item = Row<'static>> {
    matrix_rows("p1b_rows", P1B_ROWS)
}

#[allow(dead_code)]
pub fn p1c_rows() -> impl Iterator<Item = Row<'static>> {
    matrix_rows("p1c_rows", include_str!("../fixtures/p1c_rows/rows.tsv"))
}

#[allow(dead_code)]
pub fn p1d_rows() -> impl Iterator<Item = Row<'static>> {
    matrix_rows("p1d_rows", include_str!("../fixtures/p1d_rows/rows.tsv"))
}

fn matrix_rows(matrix: &'static str, tsv: &'static str) -> impl Iterator<Item = Row<'static>> {
    tsv.lines().skip(1).map(move |line| {
        let [id, fixture, ids, warnings, truth, class] =
            <[&str; 6]>::try_from(line.split('\t').collect::<Vec<_>>()).unwrap();
        Row {
            matrix,
            id,
            fixture,
            operations: ids.split(';').filter(|id| id.starts_with("op:")).collect(),
            reason: truth.strip_prefix("loss:"),
            fields: ids
                .split(';')
                .filter(|id| id.starts_with("field:"))
                .collect(),
            warnings: warnings
                .split(';')
                .filter(|warning| !warning.is_empty())
                .collect(),
            class,
        }
    })
}

pub fn matrix_file(path: &str) -> PathBuf {
    matrix_path("ids_matrix", path)
}

pub fn matrix_path(matrix: &str, path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(matrix)
        .join(path)
}

pub fn fixture(row: &Row, filename: Option<&str>) -> Vec<u8> {
    let path = matrix_path(row.matrix, row.fixture);
    fs::read(filename.map_or(path.clone(), |name| path.with_file_name(name))).unwrap()
}
