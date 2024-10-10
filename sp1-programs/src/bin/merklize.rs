#![no_main]
sp1_zkvm::entrypoint!(main);
use mmr_sp1_programs::MerklizeProgramParams;
use rust_mmr::MMR;

pub fn main() {
    let MerklizeProgramParams { leaves } = sp1_zkvm::io::read();
    let mmr = MMR::from_leaves(&leaves);
    sp1_zkvm::io::commit(&mmr.get_root());
}
