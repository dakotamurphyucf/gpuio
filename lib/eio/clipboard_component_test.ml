open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Clipboard_component
module Text = Gpuio.Clipboard.Text
module Error = Gpuio.Clipboard.Error
module Driver = Gpuio_runtime_core.Window_driver
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let text value = Text.of_string value |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

type request =
  { text : Text.t
  ; complete : (unit, Error.t) Result.t -> unit
  }

type harness =
  { driver : Driver.t
  ; text : Text.t B.Expert.Var.t
  ; disabled : bool B.Expert.Var.t
  ; active : bool B.Expert.Var.t
  ; latest : C.t option ref
  ; requests : request Queue.t
  ; copied : Text.t Queue.t
  ; mutable now : Time_ns.t
  }

let cycle t =
  Driver.cycle t.driver ~now:t.now |> ok;
  match Driver.next_message t.driver with
  | Some (W.Message.Apply tx) ->
    Driver.submitted t.driver;
    Driver.acknowledge t.driver ~revision:tx.revision |> ok
  | None -> ()
  | Some _ -> assert false
;;

let settle t =
  for _ = 1 to 4 do
    cycle t
  done
;;

let current t = Option.value_exn !(t.latest)

let run t action =
  Driver.schedule t.driver action;
  settle t
;;

let create () =
  let value = B.Expert.Var.create (text "first λ") in
  let disabled = B.Expert.Var.create false in
  let active = B.Expert.Var.create true in
  let latest = ref None in
  let requests = Queue.create () in
  let copied = Queue.create () in
  let write text =
    E.Expert.of_fun ~f:(fun ~callback ->
      Queue.enqueue requests { text; complete = callback })
  in
  let component graph =
    let open B.Let_syntax in
    match%sub B.Expert.Var.value active with
    | false -> B.return (Gpuio_bonsai.View.text "Hidden")
    | true ->
      let controller =
        C.create
          write
          ~text:(B.Expert.Var.value value)
          ~disabled:(B.Expert.Var.value disabled)
          ~on_copied:
            (B.return (fun text -> E.of_thunk (fun () -> Queue.enqueue copied text)))
          graph
      in
      let%arr controller = controller in
      latest := Some controller;
      C.view controller ()
  in
  let driver =
    Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
  in
  let t =
    { driver
    ; text = value
    ; disabled
    ; active
    ; latest
    ; requests
    ; copied
    ; now = Time_ns.epoch
    }
  in
  settle t;
  t
;;

let complete t request result = run t (E.of_thunk (fun () -> request.complete result))

let at t seconds =
  t.now <- Time_ns.add Time_ns.epoch (Time_ns.Span.of_sec seconds);
  settle t
;;

let%expect_test "copy waits for acknowledgement, suppresses duplicates and expires once" =
  let t = create () in
  run t (C.copy (current t));
  assert (C.is_busy (current t) && not (C.is_copied (current t)));
  run t (C.copy (current t));
  assert (Queue.length t.requests = 1);
  let request = Queue.dequeue_exn t.requests in
  assert (Text.equal request.text (text "first λ"));
  complete t request (Ok ());
  assert ((not (C.is_busy (current t))) && C.is_copied (current t));
  assert (Queue.length t.copied = 1);
  run t (C.copy (current t));
  assert (Queue.is_empty t.requests);
  at t 1.999;
  assert (C.is_copied (current t));
  at t 2.001;
  assert (not (C.is_copied (current t)));
  run t (C.copy (current t));
  assert (Queue.length t.requests = 1);
  Driver.close t.driver;
  [%expect {| |}]
;;

let%expect_test "value, disabled and branch retirement fence obsolete completions" =
  List.iter [ "value"; "disabled"; "branch" ] ~f:(fun change ->
    let t = create () in
    run t (C.copy (current t));
    let old = Queue.dequeue_exn t.requests in
    (match change with
     | "value" -> B.Expert.Var.set t.text (text "replacement 世界")
     | "disabled" -> B.Expert.Var.set t.disabled true
     | "branch" -> B.Expert.Var.set t.active false
     | _ -> assert false);
    settle t;
    complete t old (Ok ());
    assert (Queue.is_empty t.copied);
    B.Expert.Var.set t.disabled false;
    B.Expert.Var.set t.active true;
    settle t;
    assert (not (C.is_busy (current t) || C.is_copied (current t)));
    run t (C.copy (current t));
    let next = Queue.dequeue_exn t.requests in
    complete t old (Ok ());
    assert (C.is_busy (current t));
    complete t next (Ok ());
    assert (C.is_copied (current t) && Queue.length t.copied = 1);
    assert (Text.equal (Queue.peek_exn t.copied) (B.Expert.Var.get t.text));
    Driver.close t.driver);
  [%expect {| |}]
;;

let%expect_test "failure is visible and retry is explicit; disabled copies never submit" =
  let t = create () in
  B.Expert.Var.set t.disabled true;
  settle t;
  run t (C.copy (current t));
  assert (Queue.is_empty t.requests);
  B.Expert.Var.set t.disabled false;
  settle t;
  run t (C.copy (current t));
  complete t (Queue.dequeue_exn t.requests) (Error Busy);
  assert (Option.equal Error.equal (C.error (current t)) (Some Busy));
  assert (Queue.is_empty t.copied && Queue.is_empty t.requests);
  at t 10.;
  assert (Queue.is_empty t.requests);
  run t (C.copy (current t));
  complete t (Queue.dequeue_exn t.requests) (Ok ());
  assert (Option.is_none (C.error (current t)) && C.is_copied (current t));
  Driver.close t.driver;
  [%expect {| |}]
;;
