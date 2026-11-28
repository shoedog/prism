use super::*;

mod callees;
mod dynamic_callees;
mod operands;
mod operations;

type Refusal = (Reason, Row<'static>);

fn check_rows(table: Table, rows: &[Refusal]) {
    let failures = rows
        .iter()
        .map(|&(reason, row)| {
            std::panic::catch_unwind(|| {
                assert_eq!(reason.table(), table, "{}", reason.as_str());
                check(row, Err((table, reason)));
            })
        })
        .filter(Result::is_err)
        .count();
    assert_eq!(failures, 0, "certificate refusal rows failed");
}

#[test]
fn operation_refusals() {
    check_rows(Table::Operation, operations::ROWS);
}

#[test]
fn callee_refusals() {
    check_rows(Table::Callee, callees::ROWS);
}

#[test]
fn dynamic_callee_refusals() {
    check_rows(Table::Callee, dynamic_callees::ROWS);
}

#[test]
fn operand_refusals() {
    check_rows(Table::Operand, operands::ROWS);
}

#[test]
fn every_reason_has_a_native_refusal_row() {
    for &reason in Reason::ALL {
        assert!(
            [
                operations::ROWS,
                callees::ROWS,
                dynamic_callees::ROWS,
                operands::ROWS
            ]
            .into_iter()
            .flatten()
            .any(|&(covered, _)| covered == reason),
            "no input for {}",
            reason.as_str()
        );
    }
}
