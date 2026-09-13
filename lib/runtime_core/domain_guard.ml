open Core

type t = int

let create () = Stdlib.Domain.self_index ()

let check t =
  if t <> Stdlib.Domain.self_index ()
  then invalid_arg "GPUIO operation called outside its owning OCaml UI domain"
;;
