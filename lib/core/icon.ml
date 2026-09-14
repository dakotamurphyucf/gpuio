open Core

module Config = struct
  type t = Image.Config.t [@@deriving equal, sexp_of]

  let create ~asset ~description ?fit () =
    if Asset.Format.equal (Asset.Handle.format asset) Svg
    then Ok (Image.Config.create ~asset ~description ?fit ())
    else Or_error.error_string "icons require an SVG asset"
  ;;
end

module Expert = struct
  let image t = t
end
