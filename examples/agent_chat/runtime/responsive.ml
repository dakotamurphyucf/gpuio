open Core
module Q = Gpuio.Container_query

let at_width ~key ~height ~breakpoint ~compact ~wide =
  let compact_id = Q.Branch_id.of_string "compact" |> Or_error.ok_exn in
  let wide_id = Q.Branch_id.of_string "wide" |> Or_error.ok_exn in
  let width = Q.Range.create ~minimum:breakpoint () |> Or_error.ok_exn in
  let config =
    Q.Config.create
      ~default:compact_id
      [ Q.Rule.create ~condition:(Q.Predicate.create ~width ()) ~branch:wide_id ]
    |> Or_error.ok_exn
  in
  Gpuio_bonsai.View.container_query
    ~key:(Gpuio.Key.of_string_exn key)
    ~style:
      (Gpuio.Style.create_exn
         [ Width (Gpuio.Length.percent_exn 100.)
         ; Height (Gpuio.Length.px_exn height)
         ; Min_width (Gpuio.Length.px_exn 0.)
         ; Shrink 0.
         ])
    config
    [ compact_id, compact; wide_id, wide ]
  |> Or_error.ok_exn
;;
