open Core

type t =
  { trap : bool
  ; auto_focus : bool
  ; restore_focus : bool
  }
[@@deriving equal, sexp_of]

let create ?(trap = false) ?(auto_focus = false) ?(restore_focus = true) () =
  { trap; auto_focus; restore_focus }
;;

module Expert = struct
  let to_wire t : Gpuio_protocol.Wire.Focus_scope.t =
    { trap = t.trap; auto_focus = t.auto_focus; restore_focus = t.restore_focus }
  ;;
end
