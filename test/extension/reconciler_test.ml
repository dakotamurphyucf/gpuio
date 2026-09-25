open Core
module X = Gpuio.Extension
module R = Gpuio.Reconciler
module Wire = Gpuio_protocol.Wire

let definition name =
  let schema =
    X.Schema.create ~name ~version:1 ~fingerprint:(String.make 64 'a') |> Or_error.ok_exn
  in
  let codec =
    X.Codec.bin_prot ~max_bytes:16 Int.bin_t ~validate:(fun _ -> Ok ()) |> Or_error.ok_exn
  in
  X.Definition.create ~schema ~properties:codec ~commands:codec ~events:codec
  |> Or_error.ok_exn
;;

let%expect_test "extension updates fence callbacks and schema changes replace identity" =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = R.create window in
  let prepare ?(name = "test.counter") ?(generation = 1L) ?(disabled = false) value =
    let instance =
      X.Instance.create (definition name) ~generation ~disabled ~label:"Count" value
      |> Or_error.ok_exn
    in
    R.prepare
      reconciler
      ~theme:Gpuio.Theme.default
      (Some (Gpuio.View.extension ~on_event:Fn.id instance))
    |> Or_error.ok_exn
  in
  let operations update =
    match R.message update with
    | Some (Apply transaction) -> transaction.operations
    | _ -> []
  in
  let initial = prepare 1 in
  let node, handler =
    List.find_map_exn (operations initial) ~f:(function
      | Create (node, Extension, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  R.accept reconciler initial |> Or_error.ok_exn;
  let event handler generation signal =
    Wire.Event.Extension_event (window, node, handler, 1L, generation, signal)
  in
  let dispatch handler generation signal =
    R.dispatch reconciler (event handler generation signal)
  in
  print_s [%sexp (dispatch handler 1L (Data "\007") : int X.Event.t option)];
  let changed = prepare 2 in
  assert (
    not
      (List.exists (operations changed) ~f:(function
         | Create _ | Remove _ -> true
         | _ -> false)));
  let next_handler =
    List.find_map_exn (operations changed) ~f:(function
      | Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  R.accept reconciler changed |> Or_error.ok_exn;
  print_s [%sexp (dispatch handler 1L (Data "\007") : int X.Event.t option)];
  print_s [%sexp (dispatch next_handler 2L (Data "\007") : int X.Event.t option)];
  print_s [%sexp (dispatch next_handler 1L (Data "bad") : int X.Event.t option)];
  let disabled = prepare ~disabled:true 2 in
  let disabled_handler =
    List.find_map_exn (operations disabled) ~f:(function
      | Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  R.accept reconciler disabled |> Or_error.ok_exn;
  print_s [%sexp (dispatch disabled_handler 1L (Data "\007") : int X.Event.t option)];
  print_s
    [%sexp (dispatch disabled_handler 1L (Command_completed 4L) : int X.Event.t option)];
  let replacement = prepare ~name:"test.other" 2 in
  assert (
    List.exists (operations replacement) ~f:(function
      | Remove _ -> true
      | _ -> false));
  R.accept reconciler replacement |> Or_error.ok_exn;
  print_s [%sexp (dispatch disabled_handler 1L Mounted : int X.Event.t option)];
  R.close reconciler;
  [%expect
    {|
    ((Data 7))
    ()
    ()
    ((Failed Invalid_event))
    ()
    ((Command_completed 4))
    ()
    |}]
;;
