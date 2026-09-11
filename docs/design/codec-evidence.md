<!-- Imported from Linear 6237ff92-1371-43e4-bfd7-1fbb504089f9 on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

A 96-byte independently constructed OCaml/Rust fixture matched byte-for-byte and decoded on both sides. This is research evidence, not a complete hardened production protocol. Native-spike/wire.ml is the later experimental UI transport schema, not a finalized public wire contract.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MTU1MTE2LCJleHAiOjE3ODkxNTU0MTZ9.mmZ2kuxAPBWsrqBY-72efCBNFvcgO48mY0S8fwUdtnQ>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## codec-spike/README.md

# bin_prot interoperability experiment

Run from `/Users/dakotamurphy/gpuio-research-2026-09-10`:

```sh
dune build --root codec-spike codec.exe
cargo build --manifest-path codec-spike/rust/Cargo.toml --locked
codec-spike/_build/default/codec.exe write codec-spike/ocaml.bin
codec-spike/rust/target/debug/gpuio-codec-spike codec-spike/ocaml.bin codec-spike/rust.bin
codec-spike/_build/default/codec.exe read codec-spike/rust.bin
```

Observed result on 2026-09-10:

```text
Rust decoded OCaml fixture and produced identical bytes (96 bytes)
OCaml decoded Rust fixture successfully
```

Each language independently constructs its expected value. Rust additionally asserts byte-for-byte equality; both decoders assert full input consumption. The fixture uses OCaml `int64` for IDs and Rust `i64`; the small protocol version uses OCaml `int`/Rust `i64` under the codec's compatible integer encoding. This does not imply OCaml `int` can represent every Rust `i64`.

This checks serialization through files only. It is not an FFI implementation, a GPUI example, a malformed-input test suite, or a benchmark. The schema is illustrative, not the final GPUIO protocol. Production decoders need explicit allocation/container limits in addition to bounded input and trailing-byte rejection.

---

## codec-spike/codec.ml

```ocaml
open Core

type op =
  | Clear
  | Set_text of int64 * string
  | Set_children of int64 * int64 list
  | Listen of int64 * bool * int64 option
  | Scale of float
[@@deriving bin_io, equal]

type batch = { version : int; revision : int64; ops : op list }
[@@deriving bin_io, equal]

let fixture =
  { version = 1
  ; revision = 65536L
  ; ops =
      [ Clear
      ; Set_text (127L, "Hello λ 🦀\000")
      ; Set_children (128L, [ -129L; -128L; -1L; 0L; 255L; 256L; 32767L; 32768L; Int64.min_value; Int64.max_value ])
      ; Listen (2147483648L, true, Some 42L)
      ; Listen (0L, false, None)
      ; Scale 1.25
      ]
  }
;;

let () =
  match Sys.get_argv () with
  | [| _; "write"; path |] ->
    let buf = Bin_prot.Utils.bin_dump bin_writer_batch fixture in
    Out_channel.write_all path ~data:(Bigstring.to_string buf)
  | [| _; "read"; path |] ->
    let buf = Bigstring.of_string (In_channel.read_all path) in
    let pos_ref = ref 0 in
    let actual = bin_read_batch buf ~pos_ref in
    assert (equal_batch fixture actual);
    assert (!pos_ref = Bigstring.length buf);
    print_endline "OCaml decoded Rust fixture successfully"
  | _ -> failwith "usage: codec (write|read) path"
```

---

## codec-spike/rust/src/main.rs

```rust
use binprot::{BinProtRead, BinProtWrite};
use binprot::macros::{BinProtRead, BinProtWrite};
use std::{fs, io::Cursor};

#[derive(Debug, PartialEq, BinProtRead, BinProtWrite)]
enum Op {
    Clear,
    SetText(i64, String),
    SetChildren(i64, Vec<i64>),
    Listen(i64, bool, Option<i64>),
    Scale(f64),
}

#[derive(Debug, PartialEq, BinProtRead, BinProtWrite)]
struct Batch { version: i64, revision: i64, ops: Vec<Op> }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let expected = Batch {
        version: 1, revision: 65536,
        ops: vec![
            Op::Clear,
            Op::SetText(127, "Hello λ 🦀\0".into()),
            Op::SetChildren(128, vec![-129, -128, -1, 0, 255, 256, 32767, 32768, i64::MIN, i64::MAX]),
            Op::Listen(2147483648, true, Some(42)),
            Op::Listen(0, false, None),
            Op::Scale(1.25),
        ],
    };
    let bytes = fs::read(&args[1])?;
    let mut input = Cursor::new(&bytes);
    let actual = Batch::binprot_read(&mut input)?;
    assert_eq!(actual, expected);
    assert_eq!(input.position() as usize, bytes.len());
    let mut output = vec![];
    expected.binprot_write(&mut output)?;
    assert_eq!(bytes, output);
    fs::write(&args[2], output)?;
    println!("Rust decoded OCaml fixture and produced identical bytes ({} bytes)", bytes.len());
    Ok(())
}
```

---

## codec-spike/rust/Cargo.toml

```toml
[package]
name = "gpuio-codec-spike"
version = "0.0.0"
edition = "2021"

[dependencies]
binprot = { path = "../../sources/binprot-rs" }
```

---

## native-spike/wire.ml

```ocaml
open Core

type op =
  | Upsert of int64 * int64 * string * int64 option
  | Children of int64 * int64 list
  | Remove of int64
  | Root of int64
  | Edit of int64 * int64 * string
[@@deriving bin_io]
type batch = { version : int; base : int64; next : int64; ops : op list }
[@@deriving bin_io]
type event =
  | Ready
  | Applied of int64 * int64 * int64
  | Click of int64 * int64 * int64
  | Text of int64 * int64 * string * bool
  | Closed
  | Error of string
  | Probe of string
  | Frame of int64
[@@deriving bin_io, sexp_of]
type events = event list [@@deriving bin_io]
let encode batch = Bin_prot.Utils.bin_dump bin_writer_batch batch |> Bigstring.to_string |> Bytes.of_string
let decode bytes =
  let buffer = Bigstring.of_string (Bytes.to_string bytes) in
  let pos_ref = ref 0 in
  let events = bin_read_events buffer ~pos_ref in
  assert (!pos_ref = Bigstring.length buffer);
  events
```

