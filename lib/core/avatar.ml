open Core

module Fallback = struct
  type t = string [@@deriving equal, sexp_of]

  let create text =
    if
      String.length text > 128
      || String.is_empty (String.strip text)
      || (not (Stdlib.String.is_valid_utf_8 text))
      || String.exists text ~f:(fun c -> Char.to_int c < 32 || Char.to_int c = 127)
    then
      Or_error.error_string
        "avatar fallback must be nonblank UTF-8, at most 128 bytes, without ASCII \
         controls"
    else Ok text
  ;;

  let text t = t
end

module Config = struct
  type t =
    { asset : Asset.Handle.t option
    ; fit : Image.Fit.t
    ; fallback : Fallback.t
    ; description : Image.Description.t
    }
  [@@deriving equal, sexp_of]

  let create ?asset ?(fit = Image.Fit.Cover) ~fallback ~description () =
    { asset; fit; fallback; description }
  ;;

  let fallback t = t.fallback
  let description t = t.description
end

module Expert = struct
  let image (t : Config.t) =
    Option.map t.asset ~f:(fun asset ->
      Image.Config.create ~asset ~fit:t.fit ~description:t.description ())
  ;;

  let to_wire (t : Config.t) ~owner : Gpuio_protocol.Avatar_wire.Config.t =
    let source =
      Option.map (image t) ~f:(fun image -> (Image.Expert.to_wire image ~owner).source)
    in
    let fit : Gpuio_protocol.Image_wire.Fit.t =
      match t.fit with
      | Fill -> Fill
      | Contain -> Contain
      | Cover -> Cover
      | Scale_down -> Scale_down
      | None -> None
    in
    { source
    ; fit
    ; label = Image.Expert.label t.description
    ; fallback = Fallback.text t.fallback
    }
  ;;
end
