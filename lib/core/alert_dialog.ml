open Core

module Config = struct
  type t = Overlay.Config.t [@@deriving equal, sexp_of]

  let create ~label ?width ?dismiss_on_escape () =
    Overlay.Config.create
      ~label
      ?width
      ?dismiss_on_escape
      ~dismiss_on_outside_pointer:false
      ()
  ;;
end

module Expert = struct
  let overlay (t : Config.t) = t
end
