use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TESTS_DIR: &str = "./tests";
const TESTS_TEMPORARY_TARGET_DIR: &str = "./tests/target/";
const GCC_EXEC: &str = "./gcc_exec";
const TCC_EXEC: &str = "./a.out";
const TCC_DIR: &str = "./target/debug/tcc";
const GCC_DIR: &str = "gcc";

#[test]
fn test_valid_programs() {
    // create the tcc executable
    Command::new("cargo")
        .arg("build")
        .args(["--target-dir", TESTS_TEMPORARY_TARGET_DIR])
        .status()
        .expect("could not compile the tcc executable");

    std::env::set_current_dir(TESTS_DIR)
        .expect("could not change directory to the temporary directory");

    let mut test_programs_dir = std::env::current_dir().unwrap();
    test_programs_dir.push("programs");
    test_programs(test_programs_dir);
}

fn test_programs(dir: PathBuf) {
    let dir_entries = fs::read_dir(dir).unwrap();

    for dir_entry in dir_entries {
        let path = dir_entry.unwrap().path();
        if path.is_dir() {
            test_programs(path);
            continue;
        }

        let input_file_dir = &path.into_os_string().into_string().unwrap();
        println!("Running comparison test for the file {:?}", input_file_dir);

        // compile source code with tcc
        let tcc_exit_status = Command::new(TCC_DIR)
            .arg(input_file_dir)
            .status()
            .expect(&format!("tcc could not compile {}", input_file_dir));
        if !tcc_exit_status.success() {
            panic!(
                "tcc exited with non-zero exit code {:?}",
                tcc_exit_status.code()
            )
        }

        // compile source code with gcc
        Command::new(GCC_DIR)
            .args(["-o", GCC_EXEC])
            .arg(input_file_dir)
            .output()
            .expect(&format!("gcc could not compile {}", input_file_dir));

        let tcc_output = Command::new(TCC_EXEC)
            .output()
            .expect("could not run output generated by tcc");

        let gcc_output = Command::new(GCC_EXEC)
            .output()
            .expect("could not run output generated by gcc");

        assert_eq!(tcc_output, gcc_output);

        Command::new("rm")
            .args([GCC_EXEC, TCC_EXEC])
            .output()
            .expect("could not remove generated artifacts");
    }
}