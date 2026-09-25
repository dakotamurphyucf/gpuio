open Core

module Chrome = struct
  type t =
    | Standard
    | Hidden
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { title : string
    ; width : float
    ; height : float
    ; focus : bool
    ; chrome : Chrome.t
    ; resizable : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

let valid_title title =
  (not (String.is_empty title))
  && String.length title <= 4096
  && Stdlib.String.is_valid_utf_8 title
  && not (String.contains title '\000')
;;

let valid_size width height =
  Float.is_finite width
  && Float.is_finite height
  && Float.(width >= 1. && height >= 1. && width <= 16384. && height <= 16384.)
;;

module Command = struct
  type t =
    | Observe
    | Set_title of string
    | Resize of float * float
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited of bool
  [@@deriving bin_io, equal, sexp_of]

  let validate = function
    | Set_title title when not (valid_title title) ->
      Or_error.error_string "invalid window title"
    | Resize (width, height) when not (valid_size width height) ->
      Or_error.error_string "invalid logical window size"
    | Observe
    | Set_title _
    | Resize _
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited _ -> Ok ()
  ;;
end

module Snapshot = struct
  type t =
    { title : string
    ; x : float
    ; y : float
    ; width : float
    ; height : float
    ; content_width : float
    ; content_height : float
    ; active : bool
    ; fullscreen : bool
    ; maximized : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    String.length t.title <= 4096
    && Stdlib.String.is_valid_utf_8 t.title
    && (not (String.contains t.title '\000'))
    && List.for_all
         [ t.x; t.y; t.width; t.height; t.content_width; t.content_height ]
         ~f:Float.is_finite
    && Float.(
         t.width >= 0.
         && t.height >= 0.
         && t.content_width >= 0.
         && t.content_height >= 0.)
  ;;
end

module Backend = struct
  type t =
    | Macos
    | Wayland
    | X11
  [@@deriving bin_io, equal, sexp_of]
end

module Capabilities = struct
  type t =
    { backend : Backend.t
    ; native_quit_decision : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Closed
    | Not_ready
    | Busy
    | Invalid_request
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Observed of Snapshot.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
