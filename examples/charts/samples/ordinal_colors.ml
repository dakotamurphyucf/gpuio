open Core
module D = Gpuio.Chart_data
module S = Gpuio.Chart_style

let ok = Or_error.ok_exn
let id n = D.Datum_id.of_int64 n |> ok

let data_exn phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0. || phase > 1000.)
  then invalid_arg "Ordinal color sample phase must be finite and in [0,1000]";
  let slices =
    List.map
      [ 9L, "Research", 40.; 2L, "Build", 35.; 7L, "Review", 25. ]
      ~f:(fun (key, label, value) ->
        D.Slice.create ~id:(id key) ~label ~value:(value +. phase) |> ok)
  in
  let slices =
    if Float.to_int phase % 2 = 0
    then slices
    else List.last_exn slices :: List.take slices 2
  in
  D.pie slices |> ok
;;

let mapping ~unknown =
  S.Ordinal.create
    ~domain:[ S.Key.slice (id 2L); S.Key.slice (id 9L) ]
    ~range:[ Gpuio.Color.rgb_exn 0x2dd4bf; Gpuio.Color.rgb_exn 0x818cf8 ]
    ?unknown:(Option.some_if unknown (Gpuio.Color.rgb_exn 0xfbbf24))
    ()
  |> ok
;;
