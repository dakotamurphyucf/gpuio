open Core

let%expect_test "generic sinks preserve exact bytes and observe prior cancellation" =
  Eio_main.run (fun _ ->
    let buffer = Buffer.create 32 in
    let sink = Eio.Flow.buffer_sink buffer in
    Output.write sink "héllo 日本語";
    Output.write sink "";
    Output.write sink "\000tail";
    assert (String.equal (Buffer.contents buffer) "héllo 日本語\000tail");
    let before = Buffer.contents buffer in
    let cancelled =
      try
        Eio.Cancel.sub (fun context ->
          Eio.Cancel.cancel context Exit;
          Output.write sink "must not write");
        false
      with
      | Eio.Cancel.Cancelled _ -> true
    in
    assert cancelled;
    assert (String.equal (Buffer.contents buffer) before));
  print_endline "Exact bytes, empty write, repeated borrowing and prior cancellation";
  [%expect {| Exact bytes, empty write, repeated borrowing and prior cancellation |}]
;;
