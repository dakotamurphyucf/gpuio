open Core

module Fit = struct
  type t =
    | Fill
    | Contain
    | Cover
    | Scale_down
    | None
  [@@deriving equal, sexp_of]
end

module Description = struct
  type t =
    | Decorative
    | Label of string
  [@@deriving equal, sexp_of]

  let decorative = Decorative

  let label label =
    if
      String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
      || String.is_empty (String.strip label)
    then Or_error.error_string "image label must be bounded, nonblank UTF-8 without NUL"
    else Ok (Label label)
  ;;
end

module Config = struct
  type t =
    { asset : Asset.Handle.t
    ; description : Description.t
    ; fit : Fit.t
    }
  [@@deriving equal, sexp_of]

  let create ~asset ~description ?(fit = Fit.Contain) () = { asset; description; fit }
  let asset t = t.asset
  let fit t = t.fit
  let description t = t.description
end

module Error = struct
  type t =
    | Wrong_application
    | Released
    | Invalid_data
    | Unsupported
    | Resource_limit
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Metadata = struct
  type t =
    { width_px : int
    ; height_px : int
    ; frames : int
    }
  [@@deriving equal, sexp_of]

  let create ~width_px ~height_px ~frames =
    if
      width_px <= 0
      || height_px <= 0
      || width_px > 16384
      || height_px > 16384
      || frames <= 0
      || frames > 120
    then Or_error.error_string "invalid image pixel dimensions or frame count"
    else if width_px * height_px * 4 * frames > 64 * 1024 * 1024
    then Or_error.error_string "image frames exceed the decoded pixel budget"
    else Ok { width_px; height_px; frames }
  ;;

  let width_px t = t.width_px
  let height_px t = t.height_px
  let frames t = t.frames
end

module State = struct
  type t =
    | Loading
    | Ready of Metadata.t
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end

module Expert = struct
  module Wire = Gpuio_protocol.Wire.Image

  let error_to_wire : Error.t -> Wire.Error.t = function
    | Wrong_application -> Wrong_application
    | Released -> Released
    | Invalid_data -> Invalid_data
    | Unsupported -> Unsupported
    | Resource_limit -> Resource_limit
    | Native_failure -> Native_failure
  ;;

  let error_of_wire : Wire.Error.t -> Error.t = function
    | Wrong_application -> Wrong_application
    | Released -> Released
    | Invalid_data -> Invalid_data
    | Unsupported -> Unsupported
    | Resource_limit -> Resource_limit
    | Native_failure -> Native_failure
  ;;

  let state_of_wire : Wire.State.t -> State.t Or_error.t = function
    | Loading -> Ok Loading
    | Failed error -> Ok (Failed (error_of_wire error))
    | Ready metadata ->
      let open Or_error.Let_syntax in
      let%bind width_px =
        Int64.to_int metadata.width_px
        |> Or_error.of_option ~error:(Core.Error.of_string "image width out of range")
      in
      let%bind height_px =
        Int64.to_int metadata.height_px
        |> Or_error.of_option ~error:(Core.Error.of_string "image height out of range")
      in
      let%bind frames =
        Int64.to_int metadata.frames
        |> Or_error.of_option
             ~error:(Core.Error.of_string "image frame count out of range")
      in
      let%map metadata = Metadata.create ~width_px ~height_px ~frames in
      State.Ready metadata
  ;;

  let label = function
    | Description.Decorative -> None
    | Label label -> Some label
  ;;

  let check_owner t ~owner =
    if Asset.Expert.belongs_to (Config.asset t) ~owner
    then Ok ()
    else Error Error.Wrong_application
  ;;

  let to_wire t ~owner : Wire.Config.t =
    let source =
      match owner with
      | Some owner when Asset.Expert.belongs_to (Config.asset t) ~owner ->
        Wire.Source.Reference (Asset.Expert.native_id (Config.asset t))
      | Some _ | None -> Wire.Source.Unavailable (error_to_wire Wrong_application)
    in
    let fit : Wire.Fit.t =
      match Config.fit t with
      | Fill -> Fill
      | Contain -> Contain
      | Cover -> Cover
      | Scale_down -> Scale_down
      | None -> None
    in
    { source; fit; label = label (Config.description t) }
  ;;
end
