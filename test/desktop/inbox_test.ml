open Core
module Inbox = Gpuio_runtime_core.Desktop_inbox

let queued t value = assert (Inbox.Admission.equal (Inbox.push t value) Queued)

let%expect_test "readiness preserves order and duplicates; close cannot be undone" =
  let t = Inbox.create () in
  queued t "gpuio://first";
  queued t "gpuio://second";
  queued t "gpuio://first";
  assert (Option.is_none (Inbox.pop t));
  assert (Inbox.length t = 3);
  Inbox.ready t;
  Inbox.ready t;
  print_s [%sexp (Inbox.pop t : string option)];
  queued t "gpuio://fourth";
  print_s [%sexp (Inbox.pop t : string option)];
  print_s [%sexp (Inbox.pop t : string option)];
  assert (Inbox.bytes t = String.length "gpuio://fourth");
  Inbox.close t;
  Inbox.ready t;
  assert (Inbox.length t = 0 && Inbox.bytes t = 0);
  assert (Option.is_none (Inbox.pop t));
  print_s [%sexp (Inbox.push t "late" : Inbox.Admission.t)];
  [%expect
    {|
    (gpuio://first)
    (gpuio://second)
    (gpuio://first)
    Closed
    |}]
;;

let%expect_test "entry and byte backpressure reject newest without altering the queue" =
  let t = Inbox.create () in
  for i = 0 to Inbox.max_entries - 1 do
    queued t (Int.to_string i)
  done;
  assert (Inbox.Admission.equal (Inbox.push t "overflow") Full);
  Inbox.ready t;
  for i = 0 to Inbox.max_entries - 1 do
    assert (Option.equal String.equal (Inbox.pop t) (Some (Int.to_string i)))
  done;
  assert (Inbox.bytes t = 0);
  let payload = String.make Gpuio.Deep_link.max_bytes 'x' in
  for _ = 1 to Inbox.max_bytes / String.length payload do
    queued t payload
  done;
  assert (Inbox.Admission.equal (Inbox.push t "x") Full);
  assert (Inbox.Admission.equal (Inbox.push t (payload ^ "x")) Too_large);
  assert (Inbox.bytes t = Inbox.max_bytes);
  ignore (Inbox.pop t : string option);
  queued t payload;
  Inbox.close t;
  assert (Inbox.length t = 0 && Inbox.bytes t = 0);
  [%expect {| |}]
;;

let%expect_test "repeated fill and drain do not accumulate retained accounting" =
  let t = Inbox.create () in
  Inbox.ready t;
  for cycle = 1 to 10_000 do
    let value = String.make (cycle mod 1024) 'x' in
    queued t value;
    assert (Inbox.bytes t = String.length value);
    assert (Option.equal String.equal (Inbox.pop t) (Some value));
    assert (Inbox.length t = 0 && Inbox.bytes t = 0)
  done;
  [%expect {| |}]
;;
