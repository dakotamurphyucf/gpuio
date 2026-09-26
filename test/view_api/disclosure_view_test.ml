open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node slot = Gpuio_protocol.Node_id.create ~slot ~generation:1L |> ok
let handler slot = Gpuio_protocol.Handler_id.create ~slot ~generation:1L |> ok

let commit reconciler view =
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "appended disclosure kinds match independent native envelope" =
  let request =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node 0L, Accordion, "", None)
          ; Create (node 1L, Disclosure, "", None)
          ; Create (node 2L, Button, "Details", Some (handler 2L))
          ; Create (node 3L, Panel, "Details", None)
          ; Set_control (node 2L, Button false)
          ; Splice (node 1L, 0L, 0L, [ node 2L; node 3L ])
          ; Splice (node 0L, 0L, 0L, [ node 1L ])
          ; Set_root (Some (node 0L))
          ]
      }
  in
  let bytes = W.Message.encode request |> ok in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "disclosure-request.hex")
         |> String.strip)));
  print_endline "Panel=42, Disclosure=43, Accordion=44; existing button intents reused";
  [%expect {| Panel=42, Disclosure=43, Accordion=44; existing button intents reused |}]
;;

let%expect_test "retained and unmounted bodies have distinct native identity lifetimes" =
  List.iter [ Content_policy.Retain; Unmount ] ~f:(fun hidden ->
    let reconciler = Reconciler.create window in
    let make expanded action =
      View.disclosure
        ~key:(Key.of_string_exn "section")
        ~label:"Details"
        ~expanded
        ~hidden
        ~on_toggle:(fun () -> action)
        [ View.text ~key:(Key.of_string_exn "body") "Stable body" ]
    in
    let initial = commit reconciler (make true `First) in
    let trigger, handler =
      List.find_map_exn initial ~f:(function
        | W.Op.Create (n, Button, _, Some h) -> Some (n, h)
        | _ -> None)
    in
    let old_body =
      List.find_map_exn initial ~f:(function
        | W.Op.Create (n, Text, "Stable body", _) -> Some n
        | _ -> None)
    in
    let closed = commit reconciler (make false `Latest) in
    assert (
      Option.equal
        [%equal: [ `First | `Latest ]]
        (Reconciler.dispatch reconciler (W.Event.Press (window, trigger, handler, 1L)))
        (Some `Latest));
    let reopened = commit reconciler (make true `Latest) in
    let removed =
      List.exists closed ~f:(function
        | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id old_body
        | _ -> false)
    in
    let created =
      List.find_map reopened ~f:(function
        | W.Op.Create (n, Text, "Stable body", _) -> Some n
        | _ -> None)
    in
    (match hidden with
     | Retain ->
       assert (not removed);
       assert (Option.is_none created)
     | Unmount ->
       assert removed;
       assert (not (Gpuio_protocol.Node_id.equal old_body (Option.value_exn created))));
    print_s
      [%sexp
        (hidden : Content_policy.t), (removed : bool), (Option.is_some created : bool)]);
  [%expect
    {|
    (Retain false false)
    (Unmount true true)
    |}]
;;

let%expect_test
    "accordion builds collapsed content only under Retain and emits ordered toggle \
     intents"
  =
  let id s = Choice.Id.of_string s |> ok in
  let items =
    List.map [ "a"; "b" ] ~f:(fun label -> Choice.create ~id:(id label) ~label () |> ok)
    |> Choice.Collection.create
    |> ok
  in
  let model = Disclosure.create ~items ~mode:Multiple ~expanded:[ id "a" ] () |> ok in
  List.iter [ Content_policy.Retain; Unmount ] ~f:(fun hidden ->
    let built = ref [] in
    let view =
      View.accordion
        ~model
        ~hidden
        ~on_request:Fn.id
        ~content:(fun id ->
          built := Choice.Id.to_string id :: !built;
          [ View.text "body" ])
        ()
    in
    let reconciler = Reconciler.create window in
    let ops = commit reconciler view in
    let trigger, handler =
      List.find_map_exn ops ~f:(function
        | W.Op.Create (n, Button, "a", Some h) -> Some (n, h)
        | _ -> None)
    in
    let request =
      Reconciler.dispatch reconciler (W.Event.Press (window, trigger, handler, 1L))
      |> Option.value_exn
    in
    let latest =
      Disclosure.apply_request (Disclosure.apply_request model request) request
    in
    assert (Disclosure.equal model latest);
    print_s [%sexp (hidden : Content_policy.t), (List.rev !built : string list)]);
  [%expect
    {|
    (Retain (a b))
    (Unmount (a))
    |}]
;;
