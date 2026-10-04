open Core

(** Declarative native window gestures. These do not send pointer events through
    OCaml and do not create independent focus or task ownership. *)
module Edge : sig
  type t =
    | Top
    | Bottom
    | Left
    | Right
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

type t =
  | Title_bar
  | Exclude
  | Resize of Edge.t
[@@deriving equal, sexp_of]

(** Title_bar requests native movement on a primary drag and the platform's
    title-bar double-click action. Exclude protects nested interactive content.
    Resize requests an OS edge/corner gesture on Linux; compositor constraints
    still apply. It is inert on macOS, whose pinned GPUI backend delegates resizing
    to the native window border. It is also inert on non-resizable windows.
    Use with Custom chrome, ordinary View layout and explicit close decisions. *)
module Expert : sig
  val to_wire : t -> Gpuio_protocol.Window_region_wire.t
end
