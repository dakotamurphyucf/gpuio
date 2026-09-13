open Core

let validate_label value =
  if
    String.length value > 4096
    || String.is_empty (String.strip value)
    || String.contains value '\000'
    || not (Stdlib.String.is_valid_utf_8 value)
  then
    Or_error.error_string "file dialog labels must be bounded nonblank UTF-8 without NUL"
  else Ok ()
;;

module Open = struct
  module Selection = struct
    type t =
      | Files
      | Directories
      | Files_and_directories
    [@@deriving equal, sexp_of]
  end

  type t =
    { selection : Selection.t
    ; multiple : bool
    ; title : string
    ; accept_label : string
    ; directory : File_path.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(selection = Selection.Files)
        ?(multiple = false)
        ?(title = "Open")
        ?(accept_label = "Open")
        ?directory
        ()
    =
    let open Or_error.Let_syntax in
    let%bind () = validate_label title in
    let%map () = validate_label accept_label in
    { selection; multiple; title; accept_label; directory }
  ;;

  let selection t = t.selection
  let multiple t = t.multiple
  let title t = t.title
  let accept_label t = t.accept_label
  let directory t = t.directory
end

module Save = struct
  type t =
    { directory : File_path.t
    ; suggested_name : string
    ; title : string
    ; accept_label : string
    }
  [@@deriving equal, sexp_of]

  let create ~directory ~suggested_name ?(title = "Save") ?(accept_label = "Save") () =
    let open Or_error.Let_syntax in
    let%bind () = validate_label title in
    let%bind () = validate_label accept_label in
    if
      String.is_empty suggested_name
      || String.length suggested_name > 255
      || String.contains suggested_name '/'
      || String.contains suggested_name '\000'
      || String.equal suggested_name "."
      || String.equal suggested_name ".."
      || not (Stdlib.String.is_valid_utf_8 suggested_name)
    then
      Or_error.error_string
        "suggested name must be one UTF-8 filename of 1..255 bytes without slash or NUL"
    else Ok { directory; suggested_name; title; accept_label }
  ;;

  let directory t = t.directory
  let suggested_name t = t.suggested_name
  let title t = t.title
  let accept_label t = t.accept_label
end

module Error = Gpuio_protocol.Wire.File_dialog.Error

module Request = struct
  type t =
    | Open of Open.t
    | Save of Save.t
  [@@deriving equal, sexp_of]
end

module Expert = struct
  module Wire = Gpuio_protocol.Wire.File_dialog

  let to_wire = function
    | Request.Open config ->
      let selection : Wire.Selection.t =
        match Open.selection config with
        | Files -> Files
        | Directories -> Directories
        | Files_and_directories -> Files_and_directories
      in
      Wire.Config.Open
        { selection
        ; multiple = Open.multiple config
        ; title = Open.title config
        ; accept_label = Open.accept_label config
        ; directory = Option.map (Open.directory config) ~f:File_path.to_string
        }
    | Request.Save config ->
      Wire.Config.Save
        { directory = File_path.to_string (Save.directory config)
        ; suggested_name = Save.suggested_name config
        ; title = Save.title config
        ; accept_label = Save.accept_label config
        }
  ;;

  let result_of_wire request = function
    | Wire.Result.Failed error -> Error error
    | Cancelled -> Ok None
    | Selected paths ->
      let maximum =
        match request with
        | Request.Open config -> if Open.multiple config then 128 else 1
        | Save _ -> 1
      in
      if
        List.is_empty paths
        || List.length paths > maximum
        || List.sum (module Int) paths ~f:String.length > 262_144
      then Error Error.Native_failure
      else
        List.map paths ~f:File_path.of_string
        |> Or_error.combine_errors
        |> Result.map ~f:Option.some
        |> Result.map_error ~f:(fun _ -> Error.Native_failure)
  ;;
end
