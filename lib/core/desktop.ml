open Core
module Wire = Gpuio_protocol.Desktop_wire

module Identity = struct
  type t =
    { wire : Wire.Identity.t
    ; schemes : Deep_link.Scheme.t list
    }
  [@@deriving equal, sexp_of]

  let create ~identifier ~name ?(schemes = []) () =
    let wire =
      { Wire.Identity.identifier
      ; name
      ; schemes = List.map schemes ~f:Deep_link.Scheme.to_string
      }
    in
    if Wire.Identity.valid wire
    then Ok { wire; schemes }
    else
      Or_error.error_string
        "invalid desktop identity, display name or scheme declarations"
  ;;

  let identifier t = t.wire.identifier
  let name t = t.wire.name
  let schemes t = t.schemes
end

module Capabilities = Wire.Capabilities
module Error = Wire.Error

module Event = struct
  type t =
    | Link of Deep_link.t
    | Rejected_link of
        { input : string
        ; reason : Deep_link.Error.t
        }
    | Overflow of int64
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let identity_to_wire (t : Identity.t) = t.wire
  let capabilities_of_wire t = t
  let error_of_wire t = t
end
