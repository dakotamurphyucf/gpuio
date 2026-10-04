open Core

module Chrome = struct
  type t =
    | Standard
    | Hidden
    | Custom
  [@@deriving bin_io, equal, sexp_of]
end

module Frame = struct
  type t =
    { shadow_size : float
    ; resize_hit_size : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let default = { shadow_size = 20.; resize_hit_size = 4. }

  let valid t =
    Float.is_finite t.shadow_size
    && Float.is_finite t.resize_hit_size
    && Float.(
         t.shadow_size >= 0.
         && t.shadow_size <= 128.
         && t.resize_hit_size >= 0.5
         && t.resize_hit_size <= 32.)
  ;;
end

module Config = struct
  type t =
    { title : string
    ; width : float
    ; height : float
    ; focus : bool
    ; chrome : Chrome.t
    ; resizable : bool
    ; frame : Frame.t
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

module Document = struct
  type t =
    { path : string option
    ; edited : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Option.for_all t.path ~f:Desktop_wire.Request.valid_path
end

let max_selection_bytes = 262_144
let default_selection_bytes = 65_536

module Command = struct
  type t =
    | Observe
    | Set_title of string
    | Resize of float * float
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited of bool
    | Set_document of Document.t
    | Minimize
    | Focused_input
    | Has_text_selection
    | Selected_text of int64
    | Clear_text_selection
    | End_text_selection
  [@@deriving bin_io, equal, sexp_of]

  let validate = function
    | Selected_text maximum
      when Int64.(maximum < 0L || maximum > of_int max_selection_bytes) ->
      Or_error.error_string "invalid selected-text byte limit"
    | Set_title title when not (valid_title title) ->
      Or_error.error_string "invalid window title"
    | Resize (width, height) when not (valid_size width height) ->
      Or_error.error_string "invalid logical window size"
    | Set_document document when not (Document.valid document) ->
      Or_error.error_string "invalid represented document path"
    | Observe
    | Set_title _
    | Resize _
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited _
    | Set_document _
    | Minimize
    | Focused_input
    | Has_text_selection
    | Selected_text _
    | Clear_text_selection
    | End_text_selection -> Ok ()
  ;;
end

module Tiling = struct
  type t =
    { top : bool
    ; right : bool
    ; bottom : bool
    ; left : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Decorations = struct
  type t =
    | Server
    | Client of Tiling.t
  [@@deriving bin_io, equal, sexp_of]
end

module Controls = struct
  type t =
    { fullscreen : bool
    ; maximize : bool
    ; minimize : bool
    ; window_menu : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Presentation = struct
  type t =
    { decorations : Decorations.t
    ; controls : Controls.t
    ; resizable : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = (not t.controls.maximize) || t.resizable
end

module Appearance = struct
  type t =
    | Light
    | Vibrant_light
    | Dark
    | Vibrant_dark
  [@@deriving bin_io, equal, sexp_of]

  let is_dark = function
    | Light | Vibrant_light -> false
    | Dark | Vibrant_dark -> true
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
    ; document : Document.t option
    ; presentation : Presentation.t
    ; appearance : Appearance.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Presentation.valid t.presentation
    && Option.for_all t.document ~f:Document.valid
    && String.length t.title <= 4096
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
    | Unsupported
    | Limit_exceeded
  [@@deriving bin_io, equal, sexp_of]
end

module Input_kind = struct
  type t =
    | Input
    | Textarea
    | Combobox
    | Otp
    | Number
    | Color
    | Command_palette
  [@@deriving bin_io, equal, sexp_of]
end

module Input = struct
  type t =
    { node : Node_id.t
    ; kind : Input_kind.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Observed of Snapshot.t
    | Failed of Error.t
    | Focused_input of Input.t option
    | Selection_present of bool
    | Selected_text of string
    | Selection_updated
  [@@deriving bin_io, equal, sexp_of]
end
