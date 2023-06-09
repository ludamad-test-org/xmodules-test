use bindgen::BindgenError;
use color_eyre::{config::HookBuilder, eyre::Result};
use pkg_config::Error;
use std::{env, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum BuildError {
    #[error("Barretenberg could not be found because {0} was set.")]
    PkgConfigDisabled(String),
    #[error("Failed to locate correct Barretenberg. {0}.")]
    PkgConfigProbe(String),
    #[error("{0}")]
    PkgConfigGeneric(String),

    #[error("Clang encountered an error during rust-bindgen: {0}.")]
    BindgenErrorClangDiagnostic(String),
    #[error("Encountered a rust-bindgen error: {0}.")]
    BindgenGeneric(String),
    #[error("Failed to write {0} with rust-bindgen.")]
    BindgenWrite(String),
}

// Useful for printing debugging messages during the build
// macro_rules! p {
//     ($($tokens: tt)*) => {
//         println!("cargo:warning={}", format!($($tokens)*))
//     }
// }

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // Register a eyre hook to display more readable failure messages to end-users
    let (_, eyre_hook) = HookBuilder::default()
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;

    let lib = pkg_config::Config::new()
        .range_version("0.1.0".."0.2.0")
        .probe("barretenberg")
        .map_err(|err| match err {
            Error::EnvNoPkgConfig(val) => BuildError::PkgConfigDisabled(val),
            Error::ProbeFailure {
                name: _,
                command: _,
                ref output,
            } => BuildError::PkgConfigProbe(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ),
            err => BuildError::PkgConfigGeneric(format!("{err}")),
        })?;

    link_lib_omp();

    let include_args = lib
        .include_paths
        .iter()
        .map(|include| format!("-I{}", include.to_string_lossy()));

    // Generate bindings from a header file and place them in a bindings.rs file
    let bindings = bindgen::Builder::default()
        // Clang args so that we can compile C++ with C++20
        .clang_args(&["-std=gnu++20", "-xc++"])
        .clang_args(include_args)
        .header_contents(
            "wrapper.hpp",
            r#"
            #include <barretenberg/dsl/acir_proofs/c_bind.hpp>
            #include <barretenberg/crypto/blake2s/c_bind.hpp>
            #include <barretenberg/crypto/pedersen_commitment/c_bind.hpp>
            #include <barretenberg/crypto/schnorr/c_bind.hpp>
            #include <barretenberg/ecc/curves/bn254/scalar_multiplication/c_bind.hpp>
            "#,
        )
        .allowlist_function("blake2s_to_field")
        .allowlist_function("acir_proofs_get_solidity_verifier")
        .allowlist_function("acir_proofs_get_exact_circuit_size")
        .allowlist_function("acir_proofs_get_total_circuit_size")
        .allowlist_function("acir_proofs_init_proving_key")
        .allowlist_function("acir_proofs_init_verification_key")
        .allowlist_function("acir_proofs_new_proof")
        .allowlist_function("acir_proofs_verify_proof")
        .allowlist_function("pedersen_plookup_compress_fields")
        .allowlist_function("pedersen_plookup_compress")
        .allowlist_function("pedersen_plookup_commit")
        .allowlist_function("new_pippenger")
        .allowlist_function("compute_public_key")
        .allowlist_function("construct_signature")
        .allowlist_function("verify_signature")
        .generate()
        .map_err(|err| match err {
            BindgenError::ClangDiagnostic(msg) => {
                BuildError::BindgenErrorClangDiagnostic(msg.trim().to_string())
            }
            err => BuildError::BindgenGeneric(format!("{err}").trim().to_string()),
        })?;

    println!("cargo:rustc-link-lib=static=barretenberg");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindgen_file = out_path.join("bindings.rs");
    bindings
        .write_to_file(&bindgen_file)
        .map_err(|_| BuildError::BindgenWrite(bindgen_file.to_string_lossy().to_string()).into())
}

fn link_lib_omp() {
    // If we are using clang from brew, so we need to tell the linker where to search for libomp
    if let Some(brew_prefix) = find_brew_prefix() {
        println!("cargo:rustc-link-search={brew_prefix}/opt/libomp/lib");
    }
    println!("cargo:rustc-link-lib=omp");
}

fn find_brew_prefix() -> Option<String> {
    let output = std::process::Command::new("brew")
        .arg("--prefix")
        .stdout(std::process::Stdio::piped())
        .output();

    match output {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(stdout) => Some(stdout.trim().to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}
