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

let choice_id text = Choice.Id.of_string text |> ok

let rich_model () =
  let items =
    List.map
      [ "first", false; "locked", true; "last", false ]
      ~f:(fun (label, disabled) ->
        Choice.create ~id:(choice_id label) ~label ~disabled () |> ok)
    |> Choice.Collection.create
    |> ok
  in
  Disclosure.create ~items ~mode:Multiple ~expanded:[ choice_id "first" ] () |> ok
;;

let rich_view ?(hidden = Content_policy.Retain) model labels =
  View.accordion_with_labels
    ~model
    ~labels
    ~hidden
    ~on_request:Fn.id
    ~content:(fun id ->
      [ View.text ~key:(Key.of_string_exn "body") ("Body " ^ Choice.Id.to_string id) ])
    ()
;;

let%expect_test "rich accordion headings preserve triggers, bodies and current callbacks" =
  let model = rich_model () in
  let labels suffix =
    [ choice_id "first", View.row [ View.text "◆"; View.text suffix ] ]
  in
  let r = Reconciler.create window in
  let initial = commit r (rich_view model (labels "Initial") |> ok) in
  let trigger, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (n, Button, "first", Some h) -> Some (n, h)
      | _ -> None)
  in
  assert (
    List.count initial ~f:(function
      | W.Op.Set_accessibility (_, Some { role = Some (Heading 3); _ }) -> true
      | _ -> false)
    = 3);
  assert (
    List.count initial ~f:(function
      | W.Op.Create (_, Button, _, Some _) -> true
      | _ -> false)
    = 2);
  let request =
    Reconciler.dispatch r (W.Event.Press (window, trigger, handler, 1L))
    |> Option.value_exn
  in
  let closed = Disclosure.apply_request model request in
  let updates = commit r (rich_view closed (labels "Changed") |> ok) in
  assert (
    not
      (List.exists updates ~f:(function
         | W.Op.Remove _ -> true
         | _ -> false)));
  assert (
    Option.equal
      Disclosure.Request.equal
      (Reconciler.dispatch r (W.Event.Press (window, trigger, handler, 1L)))
      (Some request));
  ignore
    (commit r (rich_view (Disclosure.with_disabled closed true) (labels "Disabled") |> ok)
     : W.Op.t list);
  assert (
    Option.is_none (Reconciler.dispatch r (W.Event.Press (window, trigger, handler, 1L))));
  print_endline
    "three headings, two enabled triggers; labels update without replacing retained \
     bodies; disable fences queued presses";
  [%expect
    {| three headings, two enabled triggers; labels update without replacing retained bodies; disable fences queued presses |}]
;;

let%expect_test
    "rich accordion rejects unknown, duplicate, interactive and excessive labels"
  =
  let model = rich_model () in
  let first = choice_id "first" in
  List.iter
    [ [ choice_id "missing", View.text "Unknown" ]
    ; [ first, View.text "First"; first, View.text "Duplicate" ]
    ; [ ( first
        , View.button
            ~on_click:(fun () -> Disclosure.Request.Toggle first)
            "Nested action" )
      ]
    ; [ first, View.column (List.init 4096 ~f:(fun _ -> View.text "Too many")) ]
    ]
    ~f:(fun labels -> assert (Result.is_error (rich_view model labels)));
  List.iter [ 0; 7 ] ~f:(fun heading_level ->
    assert (
      Result.is_error
        (View.accordion_with_labels
           ~heading_level
           ~model
           ~labels:[]
           ~hidden:Retain
           ~on_request:Fn.id
           ~content:(fun _ -> [])
           ())));
  print_endline
    "invalid label ownership, focus descendants, aggregate budget and heading levels \
     rejected";
  [%expect
    {| invalid label ownership, focus descendants, aggregate budget and heading levels rejected |}]
;;

let%expect_test "rich accordion Unmount skips collapsed content construction" =
  let model = rich_model () in
  let built = ref [] in
  let view =
    View.accordion_with_labels
      ~model
      ~labels:[]
      ~hidden:Unmount
      ~on_request:Fn.id
      ~content:(fun id ->
        built := Choice.Id.to_string id :: !built;
        [ View.text "body" ])
      ()
    |> ok
  in
  ignore (commit (Reconciler.create window) view : W.Op.t list);
  print_s [%sexp (!built : string list)];
  [%expect {| (first) |}]
;;

let%expect_test "public rich accordion transaction sequence" =
  let model = rich_model () in
  let closed = Disclosure.apply_request model (Toggle (choice_id "first")) in
  let reconciler = Reconciler.create window in
  List.iteri [ model; closed; model ] ~f:(fun index model ->
    let labels =
      [ choice_id "first", View.row [ View.text "◆"; View.text "First section" ] ]
    in
    let view =
      View.accordion_with_labels
        ~model
        ~labels
        ~hidden:Retain
        ~on_request:(fun _ -> ())
        ~content:(fun id ->
          if Choice.Id.equal id (choice_id "first")
          then
            [ View.text_input
                ~controller:(Key.of_string_exn "accordion-draft")
                ~style:
                  (Style.create_exn
                     [ Width (Length.px_exn 280.); Height (Length.px_exn 72.) ])
                ~config:
                  (Text_input.Config.create ~mode:Multiline ~label:"Accordion draft" ()
                   |> ok)
                ~initial_text:"Retained draft"
                ~on_event:(fun _ -> ())
                ()
              |> ok
            ]
          else [ View.text "Panel content" ])
        ()
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update |> Option.value_exn in
    Reconciler.accept reconciler update |> ok;
    let bytes = W.Message.encode message |> ok in
    let hex =
      String.to_list bytes
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    let expected =
      Eio_main.run (fun env ->
        Eio.Path.load
          Eio.Path.(Eio.Stdenv.fs env / sprintf "disclosure-rich-%d.hex" index)
        |> String.strip)
    in
    assert (String.equal hex expected));
  [%expect {| |}]
;;
