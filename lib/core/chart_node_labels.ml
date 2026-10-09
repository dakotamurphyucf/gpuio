open Core
module Wire = Gpuio_protocol.Chart_node_labels_wire

module Line = struct
  type t =
    { text : string
    ; color : Color.t option
    ; font_size : float option
    }
  [@@deriving equal, sexp_of]

  let create ?color ?font_size text =
    if Wire.Line.valid { text; color = None; font_size }
    then Ok { text; color; font_size }
    else
      Or_error.error_string
        "chart label requires at most 256 UTF-8 bytes without ASCII controls and font \
         size in [8,32]"
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%map color =
      Option.value_map t.color ~default:(Ok None) ~f:(fun c ->
        Theme.resolve theme c |> Or_error.map ~f:Option.some)
    in
    { Wire.Line.text = t.text; color; font_size = t.font_size }
  ;;
end

module Node = struct
  type t =
    { node : Chart_data.Node_id.t
    ; lines : Line.t list
    }
  [@@deriving equal, sexp_of]

  let create ~node lines =
    if List.length lines <= 4
    then Ok { node; lines }
    else Or_error.error_string "chart node label requires at most four lines"
  ;;
end

type t = Node.t list [@@deriving equal, sexp_of]

let create nodes =
  if List.length nodes > 128
  then Or_error.error_string "chart labels require at most 128 node entries"
  else if
    List.contains_dup
      (List.map nodes ~f:(fun n -> n.Node.node))
      ~compare:Chart_data.Node_id.compare
  then Or_error.error_string "chart label node IDs must be unique"
  else if
    List.sum
      (module Int)
      nodes
      ~f:(fun n ->
        List.sum (module Int) n.Node.lines ~f:(fun l -> String.length l.Line.text))
    > 32768
  then Or_error.error_string "chart node label text exceeds 32 KiB"
  else Ok nodes
;;

let empty = []

module Expert = struct
  let to_wire t ~theme =
    List.map t ~f:(fun n ->
      let open Or_error.Let_syntax in
      let%map lines =
        List.map n.Node.lines ~f:(fun l -> Line.resolve l theme) |> Or_error.all
      in
      { Wire.Node.node = Chart_data.Node_id.to_int64 n.node; lines })
    |> Or_error.all
  ;;
end
