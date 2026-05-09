use monstertruck_meshing::prelude::*;
use monstertruck_step::{load::*, save::*};
use monstertruck_topology::shell::ShellCondition;

const STEP_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/step/");

const STEP_FILES: &[&str] = &[
    "occt-cone.step",
    "occt-cube.step",
    "occt-cylinder.step",
    "occt-sphere.step",
    "occt-torus.step",
    "abc-0000.step",
    "abc-0006.step",
    "abc-0008.step",
    "abc-0035.step",
];

#[test]
fn ioi() {
    let closed_shell_count = STEP_FILES
        .iter()
        .map(|file_name| {
            println!("{file_name}");
            let input = [STEP_DIRECTORY, file_name].concat();
            let step_string = std::fs::read_to_string(input).unwrap();
            let table = Table::from_step(&step_string).unwrap();
            table
                .shell
                .values()
                .map(|step_shell| {
                    let cshell = table.to_compressed_trimmed_shell(step_shell).unwrap();
                    let step_string =
                        CompleteStepDisplay::new(StepModel::from(&cshell), Default::default())
                            .to_string();
                    println!("{step_string}");
                    let table = Table::from_step(&step_string).unwrap();
                    table
                        .shell
                        .values()
                        .filter(|step_shell| {
                            let cshell = table.to_compressed_trimmed_shell(*step_shell).unwrap();
                            let bdb = cshell
                                .robust_triangulation(0.01)
                                .to_polygon()
                                .bounding_box();
                            let diag = bdb.max() - bdb.min();
                            let r = diag.x.min(diag.y).min(diag.z);
                            let mut poly = cshell.robust_triangulation(0.01 * r).to_polygon();
                            poly.put_together_same_attrs(TOLERANCE * 50.0)
                                .remove_degenerate_faces();
                            poly.shell_condition() == ShellCondition::Closed
                        })
                        .count()
                })
                .sum::<usize>()
        })
        .sum::<usize>();
    assert!(closed_shell_count > 0);
}
