open Core
module W = Gpuio_protocol.Accessibility_wire
module Role = W.Role
module Live = W.Live

module Field = struct
  type t = W.Field.t [@@deriving equal, sexp_of]

  let create ~label ?help ?error ?(required = false) () =
    let field = { W.Field.label; help; error; required } in
    if W.Field.valid field
    then Ok field
    else
      Or_error.error_string
        "field text must be nonempty UTF-8 without NUL, at most 4096 bytes"
  ;;

  let label t = t.W.Field.label
  let help t = t.W.Field.help
  let error t = t.W.Field.error
  let is_required t = t.W.Field.required
end

type t = W.Config.t [@@deriving equal, sexp_of]

let create ?role ?label ?description ?live () =
  let live =
    Option.value
      live
      ~default:
        (match role with
         | Some Role.Status -> Live.Polite
         | Some Alert -> Assertive
         | None
         | Some
             ( Group
             | Label
             | Link
             | Separator
             | Description_list
             | Term
             | Definition
             | Image
             | Heading _ ) -> Off)
  in
  let config = { W.Config.role; label; description; live; field = None } in
  if W.Config.valid config
  then Ok config
  else Or_error.error_string "invalid accessibility text or heading level"
;;

let field field =
  { W.Config.role = None
  ; label = None
  ; description = None
  ; live = Off
  ; field = Some field
  }
;;

module Expert = struct
  let to_wire t = t
end
