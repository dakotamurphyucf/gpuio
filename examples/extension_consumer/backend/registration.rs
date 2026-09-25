// Generated static registration. No OCaml values are stored in components.
#[ocaml_interop::export]
pub fn gpuio_gpuio_counter_backend_initialize(_cr: &mut ocaml_interop::OCamlRuntime, _unit: ocaml_interop::OCaml<()>) {
    gpuio_native::extensions::install([component_0::factory()])
        .expect("incompatible or duplicate statically linked native components");
}
