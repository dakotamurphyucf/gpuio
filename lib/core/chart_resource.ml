open Core

module Owner = struct
  type t = unit ref

  let create () = ref ()
  let equal = phys_equal
  let sexp_of_t _ = Sexp.Atom "<application>"
end

type t =
  { owner : Owner.t
  ; id : Gpuio_protocol.Resource_id.t
  }
[@@deriving equal, sexp_of]

module Expert = struct
  module Owner = Owner

  let handle ~owner id = { owner; id }
  let belongs_to t ~owner = Owner.equal t.owner owner
  let native_id t = t.id
end
