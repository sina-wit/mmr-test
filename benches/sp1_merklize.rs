use alloy_primitives::keccak256;
use mmr_sp1_programs::MerklizeProgramParams;
use num_format::{Locale, ToFormattedString};
use sp1_build::{build_program_with_args, BuildArgs};
use sp1_prover::utils::get_cycles;
use sp1_sdk::SP1Stdin;
use std::{
    env,
    error::Error,
    fmt,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    // Build the ELF.
    let program_path_fragment = "sp1-programs";
    let program_crate_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(program_path_fragment);
    let output_path = program_crate_path.join("elfs");
    let program_name = "merklize";
    let args = BuildArgs {
        binary: program_name.to_string(),
        locked: true,
        output_directory: output_path.to_str().unwrap().to_string(),
        ..Default::default()
    };
    build_program_with_args(program_path_fragment, args);

    // Get the ELF.
    let elf = get_elf_bytes(output_path.join(program_name).as_path());

    // // Run some iterations with various inputs set.
    let bench_results = (0..16)
        .map(|i| {
            let num_leaves = 2_u64.pow(i as u32);
            let leaves = (0..num_leaves)
                .map(|leaf_idx| keccak256(leaf_idx.to_ne_bytes()))
                .collect();
            let mut stdin = SP1Stdin::new();
            stdin.write(&MerklizeProgramParams { leaves });
            let cycles = get_cycles(&elf, &stdin);
            MerklizeBenchResult {
                iteration: i,
                args: vec![format!("2^{} = {} leaves", i, num_leaves)],
                total_cycles: cycles,
                cycles_per_leaf: cycles / num_leaves,
            }
        })
        .collect::<Vec<_>>();
    let bench_results = MerklizeBenchResults(bench_results);
    // Print the results as a table.
    println!("{}", bench_results);
    // Write the results as a md table in sp1-programs/bench-results/{program_name}.md
    let bench_results_path = program_crate_path
        .join("bench-results")
        .join(format!("{}.md", program_name));
    fs::create_dir_all(bench_results_path.parent().unwrap())?;
    let mut file = File::create(bench_results_path)?;
    write!(file, "{}", bench_results)?;

    Ok(())
}

fn get_elf_bytes(path: &Path) -> Vec<u8> {
    let mut buffer = Vec::new();
    File::open(path)
        .expect("file not found")
        .read_to_end(&mut buffer)
        .expect("failed to read file");
    buffer
}

struct MerklizeBenchResult {
    iteration: u64,
    args: Vec<String>,
    total_cycles: u64,
    cycles_per_leaf: u64,
}

struct MerklizeBenchResults(Vec<MerklizeBenchResult>);

impl fmt::Display for MerklizeBenchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "## Merklize Bench Results")?;
        writeln!(f, "| Iteration | Args | Total Cycles | Cycles Per Leaf |")?;
        writeln!(f, "|-----------|------|--------------|----------------|")?;
        for result in &self.0 {
            writeln!(
                f,
                "| {} | {} | {} | {} |",
                result.iteration,
                result.args.join(","),
                result.total_cycles.to_formatted_string(&Locale::en),
                result.cycles_per_leaf.to_formatted_string(&Locale::en)
            )?;
        }
        Ok(())
    }
}
