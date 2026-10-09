open Core
module Wire = Gpuio_protocol.Window_wire
module Chrome = Wire.Chrome
module Backend = Wire.Backend
module Capabilities = Wire.Capabilities
module Tiling = Wire.Tiling
module Decorations = Wire.Decorations
module Controls = Wire.Controls
module Presentation = Wire.Presentation
module Appearance = Wire.Appearance
module Snapshot = Wire.Snapshot

module Command = struct
  type t =
    | Observe
    | Set_title of string
    | Resize of float * float
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited of bool
    | Set_document of Wire.Document.t
    | Minimize
  [@@deriving equal, sexp_of]

  module Expert = struct
    let to_wire : t -> Wire.Command.t = function
      | Observe -> Observe
      | Set_title title -> Set_title title
      | Resize (width, height) -> Resize (width, height)
      | Activate -> Activate
      | Zoom -> Zoom
      | Toggle_fullscreen -> Toggle_fullscreen
      | Set_edited edited -> Set_edited edited
      | Set_document document -> Set_document document
      | Minimize -> Minimize
    ;;
  end

  let validate t = Wire.Command.validate (Expert.to_wire t)
end

module Input = struct
  module Kind = Wire.Input_kind

  type t =
    { window : Gpuio_protocol.Window_id.t
    ; input : Wire.Input.t
    }
  [@@deriving equal, sexp_of]

  let kind t = t.input.kind

  let same_owner t window node =
    Gpuio_protocol.Window_id.equal t.window window
    && Gpuio_protocol.Node_id.equal t.input.node node
  ;;

  let same_text_input t snapshot =
    (match t.input.kind with
     | Input | Textarea | Combobox -> true
     | Otp | Number | Color | Command_palette -> false)
    && same_owner t (Text_input.Expert.window snapshot) (Text_input.Expert.node snapshot)
  ;;

  let same_otp_input t snapshot =
    Kind.equal t.input.kind Otp
    && same_owner t (Otp_input.Expert.window snapshot) (Otp_input.Expert.node snapshot)
  ;;

  let same_number_input t snapshot =
    Kind.equal t.input.kind Number
    && same_owner
         t
         (Number_input.Expert.window snapshot)
         (Number_input.Expert.node snapshot)
  ;;

  let same_color_input t snapshot =
    Kind.equal t.input.kind Color
    && same_owner
         t
         (Color_input.Expert.window snapshot)
         (Color_input.Expert.node snapshot)
  ;;

  module Expert = struct
    let of_wire ~window input = { window; input }
  end
end

module Error = Wire.Error

module Document = struct
  type t =
    { path : File_path.t option
    ; edited : bool
    }
  [@@deriving equal, sexp_of]

  let create ?path ~edited () = { path; edited }
  let path t = t.path
  let edited t = t.edited

  let of_snapshot (snapshot : Snapshot.t) =
    Option.bind snapshot.document ~f:(fun document ->
      match document.path with
      | None -> Some { path = None; edited = document.edited }
      | Some path ->
        Result.ok (File_path.of_string path)
        |> Option.map ~f:(fun path -> { path = Some path; edited = document.edited }))
  ;;

  module Expert = struct
    let to_wire t =
      { Wire.Document.path = Option.map t.path ~f:File_path.to_string; edited = t.edited }
    ;;
  end
end

module Config = struct
  type t = Wire.Config.t

  let create
        ?(focus = true)
        ?(chrome = Chrome.Standard)
        ?(resizable = true)
        ?(frame = Window_frame.default)
        ~title
        ~width
        ~height
        ()
    =
    if Wire.valid_title title && Wire.valid_size width height
    then
      Ok
        { Wire.Config.title
        ; width
        ; height
        ; focus
        ; chrome
        ; resizable
        ; frame = Window_frame.Expert.to_wire frame
        }
    else Or_error.error_string "invalid window title or logical size"
  ;;

  module Expert = struct
    let to_wire t = t
  end
end

module Close_reason = struct
  type t =
    | Window_close
    | Application_quit
  [@@deriving equal, sexp_of]
end

module Close_decision = struct
  type t =
    | Allow
    | Keep_open
  [@@deriving equal, sexp_of]
end
