open Core

module Config = struct
  type t = Image.Config.t [@@deriving equal, sexp_of]

  let create ~asset ~description ?fit () =
    if Asset.Format.equal (Asset.Handle.format asset) Svg
    then Ok (Image.Config.create ~asset ~description ?fit ())
    else Or_error.error_string "icons require an SVG asset"
  ;;
end

module Decoration = struct
  type t =
    { config : Config.t
    ; style : Style.t
    }
  [@@deriving equal, sexp_of]

  let create ~asset ?(style = Style.empty) () =
    let open Or_error.Let_syntax in
    let%map config = Config.create ~asset ~description:Image.Description.decorative () in
    let defaults =
      Style.create_exn
        [ Width (Length.px_exn 16.); Height (Length.px_exn 16.); Shrink 0. ]
    in
    { config; style = Style.merge [ defaults; style ] }
  ;;
end

module Expert = struct
  let image t = t
  let decoration { Decoration.config; style } = config, style
end
