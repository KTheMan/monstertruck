use super::*;

#[test]
fn work_meter_accumulates_and_clears_on_the_current_thread() {
    take_continuity_work();
    charge_continuity_work(ContinuityWork {
        iterations: 2,
        jacobian_elements: 12,
        qr_elements: 20,
        truncated: false,
    });

    assert_eq!(
        take_continuity_work(),
        ContinuityWork {
            iterations: 2,
            jacobian_elements: 12,
            qr_elements: 20,
            truncated: false,
        }
    );
    assert_eq!(continuity_work(), ContinuityWork::default());
}

#[test]
fn budget_refusal_is_typed_and_marks_the_work_meter() {
    take_continuity_work();
    let error = ContinuityLimits::unbounded()
        .with_max_variables(3)
        .ensure_dimension(ContinuityResource::Variables, 4)
        .expect_err("the checked dimension exceeds its budget");

    assert_eq!(
        error,
        ContinuitySolveError::Truncated(ContinuityTruncated {
            resource: ContinuityResource::Variables,
            spent: 0,
            requested: 4,
            budget: 3,
        })
    );
    assert!(take_continuity_work().truncated);
}
