open Core
module P = Gpuio.Document.Profile
module Example = Gpuio_example_document

let ok = Or_error.ok_exn

let%expect_test "paired property bytes and native action events" =
  List.iter [ Example.Accent.Indigo; Amber ] ~f:(fun accent ->
    let instance = Example.instance ~accent ~generation:1L |> ok in
    let bytes = (P.Expert.to_wire instance).properties in
    print_s
      [%sexp
        (accent : Example.Accent.t)
      , (String.to_list bytes |> List.map ~f:Char.to_int : int list)]);
  let instance = Example.instance ~accent:Indigo ~generation:1L |> ok in
  List.iter [ "\001"; "\002"; "\003"; "\004"; ""; "\005"; "\001\000" ] ~f:(fun payload ->
    let wire : Gpuio_protocol.Document_profile_wire.Event.t =
      { config_epoch = 1L
      ; instance_generation = 1L
      ; source_revision = 2L
      ; source_generation = 1L
      ; signal = Data payload
      }
    in
    let event = P.Expert.event instance wire |> ok in
    print_s [%sexp (event.signal : Example.Event.t P.Signal.t)]);
  assert (Result.is_error (Example.instance ~accent:Indigo ~generation:0L));
  [%expect
    {|
    (Indigo (0))
    (Amber (1))
    (Data Inspect_code)
    (Data Summarize_table)
    (Data Open_badge)
    (Data Open_card)
    (Failed (stage Input) (error Invalid_event))
    (Failed (stage Input) (error Invalid_event))
    (Failed (stage Input) (error Invalid_event))
  |}]
;;
