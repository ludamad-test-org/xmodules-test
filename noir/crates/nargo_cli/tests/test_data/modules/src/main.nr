mod foo;
// This is a comment.
//
// `main` is the entry point to a binary
//
// You can have a `Binary` or a `Library`
// Release : 0.2
//
// To run a proof on the command line, type `cargo run prove {proof_name}`
//
// To verify that proof, type `cargo run verify {proof_name}`
fn main(x: Field, y: pub Field) {
    assert(x != foo::hello(y));
}
