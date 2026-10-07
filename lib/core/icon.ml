open Core

module Transform = struct
  type t = Gpuio_protocol.Icon_transform_wire.t [@@deriving equal, sexp_of]

  let create
        ?(scale_x = 1.)
        ?(scale_y = 1.)
        ?(rotation_degrees = 0.)
        ?(translate_x = 0.)
        ?(translate_y = 0.)
        ()
    =
    let t =
      { Gpuio_protocol.Icon_transform_wire.scale_x
      ; scale_y
      ; rotation_degrees
      ; translate_x
      ; translate_y
      }
    in
    if Gpuio_protocol.Icon_transform_wire.valid t
    then Ok t
    else
      Or_error.error_string "icon transform exceeds finite scale/angle/translation bounds"
  ;;

  let identity = create () |> Or_error.ok_exn
  let rotate_degrees rotation_degrees = create ~rotation_degrees ()
end

module Config = struct
  type t =
    { image : Image.Config.t
    ; transform : Transform.t option
    }
  [@@deriving equal, sexp_of]

  let create ~asset ~description ?fit ?transform () =
    if Asset.Format.equal (Asset.Handle.format asset) Svg
    then Ok { image = Image.Config.create ~asset ~description ?fit (); transform }
    else Or_error.error_string "icons require an SVG asset"
  ;;

  let with_transform t transform = { t with transform }
end

module Decoration = struct
  type t =
    { config : Config.t
    ; style : Style.t
    }
  [@@deriving equal, sexp_of]

  let create ~asset ?(style = Style.empty) ?transform () =
    let open Or_error.Let_syntax in
    let%map config =
      Config.create ~asset ~description:Image.Description.decorative ?transform ()
    in
    let defaults =
      Style.create_exn
        [ Width (Length.px_exn 16.); Height (Length.px_exn 16.); Shrink 0. ]
    in
    { config; style = Style.merge [ defaults; style ] }
  ;;
end

module Expert = struct
  let image { Config.image; _ } = image
  let transform { Config.transform; _ } = transform
  let transform_to_wire t = t
  let decoration { Decoration.config; style } = config, style
end
