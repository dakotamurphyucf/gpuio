open Core
module Wire = Gpuio_protocol.Chart_grid_wire

type t =
  { config : Wire.t
  ; color : Color.t option
  }
[@@deriving equal, sexp_of]

let create ?x ?y ?(dashes = []) ?(width = 1.) ?color () =
  let positions = Option.map ~f:(List.map ~f:Chart_axis.Expert.position_to_wire) in
  let config = { Wire.x = positions x; y = positions y; dashes; width; color = None } in
  if Wire.valid config
  then Ok { config; color }
  else Or_error.error_string "invalid chart grid positions, dash lengths or width"
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%map color =
      Option.value_map t.color ~default:(Ok None) ~f:(fun color ->
        Theme.resolve theme color |> Or_error.map ~f:Option.some)
    in
    { t.config with color }
  ;;
end
