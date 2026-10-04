open Core
open Gpuio
open Gpuio_protocol
module P = Gpuio.Choice_picker
module W = Choice_picker_wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let choice_id value = Choice.Id.of_string value |> ok
let window = Window_id.create ~slot:0L ~generation:1L |> ok

let options =
  Choice.Collection.create
    (List.map
       [ "a", false; "b", false; "locked", true ]
       ~f:(fun (id, disabled) ->
         Choice.create ~id:(choice_id id) ~label:id ~disabled () |> ok))
  |> ok
  |> P.Collection.flat
;;

let config ?(disabled = false) ?(selected = []) () =
  P.Config.create
    ~label:"Pick"
    ~options
    ~selected:(P.Selection.multiple (List.map selected ~f:choice_id) |> ok)
    ~search:Substring
    ~disabled
    ~clearable:true
    ()
  |> ok
;;

let observe label = function
  | P.Event.Selection_requested request ->
    label
    ^ ":"
    ^ Sexp.to_string (P.Request.sexp_of_t (P.Selection_request.request request))
  | Query_changed snapshot -> label ^ ":query:" ^ Text_input.Snapshot.text snapshot
  | Open_requested _ -> label ^ ":open"
  | Visibility _ -> label ^ ":visibility"
;;

let description ?(controller = "query") ?(initial_text = "seed") config =
  P.Description.create
    ~config
    ~query:(P.Query.create ~controller:(key controller) ~initial_text () |> ok)
    ~trigger:(View.text "Choose")
    ~footer:(View.button ~on_click:(fun () -> "footer") "Footer")
    ~options:[ choice_id "a", P.Option_content.create (View.text "Rich a") ]
    ()
  |> ok
;;

let view ?controller ?initial_text ?(label = "first") config =
  View.choice_picker
    ~key:(key "picker")
    ~on_event:(observe label)
    (description ?controller ?initial_text config)
  |> ok
;;

let accept r next =
  let update = Reconciler.prepare r ~theme:Theme.default next |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (Apply tx) -> tx
  | _ -> failwith "expected transaction"
;;

let created tx kind =
  List.find_map_exn tx.Wire.Transaction.operations ~f:(function
    | Wire.Op.Create (node, k, _, Some handler) when Wire.Kind.equal k kind ->
      Some (node, handler)
    | _ -> None)
;;

let%expect_test "picker public view routes current-model intents and query generations" =
  let r = Reconciler.create window in
  let first = accept r (Some (view (config ()))) in
  let bytes =
    Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t (Wire.Message.Apply first)
    |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let fixture =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-public-view.hex")
      |> String.strip)
  in
  assert (String.equal hex fixture);
  let root, handler = created first Choice_picker in
  let query, query_handler = created first Input in
  let snapshot : Wire.Editor.Snapshot.t =
    { revision = 3L
    ; text = "draft"
    ; selection = { anchor = 5L; head = 5L }
    ; composition = None
    ; focused = true
    }
  in
  let query_value : W.Query.t = { node = query; snapshot } in
  let event payload =
    Wire.Event.Choice_picker_event (window, root, handler, first.revision, payload)
  in
  let toggle = event (Selection_requested (Toggle "a", Some query_value)) in
  let dispatch expected event =
    assert (Option.equal String.equal (Reconciler.dispatch r event) expected)
  in
  dispatch (Some "first:(Toggle a)") toggle;
  dispatch None (event (Selection_requested (Select "a", Some query_value)));
  dispatch None (event (Selection_requested (Toggle "locked", Some query_value)));
  dispatch None (event (Selection_requested (Toggle "missing", Some query_value)));
  dispatch None (event (Selection_requested (Clear, None)));
  let changed = event (Query_changed query_value) in
  dispatch
    None
    (Wire.Event.Editor_event
       (window, query, query_handler, first.revision, Changed, snapshot));
  dispatch (Some "first:query:draft") changed;
  dispatch
    None
    (Wire.Event.Editor_event
       (window, query, query_handler, first.revision, Submitted, snapshot));
  let candidate =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (view
            ~label:"latest"
            ~initial_text:"ignored new seed"
            (config ~selected:[ "a" ] ())))
    |> ok
  in
  dispatch (Some "first:(Toggle a)") toggle;
  Reconciler.accept r candidate |> ok;
  dispatch (Some "latest:(Toggle a)") toggle;
  dispatch (Some "latest:(Toggle a)") toggle;
  dispatch (Some "latest:query:draft") changed;
  let operations =
    match Reconciler.message candidate with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  assert (
    not
      (List.exists operations ~f:(function
         | Set_text _ | Bind _ -> true
         | _ -> false)));
  ignore
    (accept r (Some (view ~label:"new-query" ~controller:"replacement" (config ())))
     : Wire.Transaction.t);
  dispatch None toggle;
  dispatch None changed;
  let disabled =
    accept r (Some (view ~controller:"replacement" (config ~disabled:true ())))
  in
  let new_handler =
    List.find_map_exn disabled.operations ~f:(function
      | Bind (node, Some handler) when Node_id.equal node root -> Some handler
      | _ -> None)
  in
  assert (not (Handler_id.equal handler new_handler));
  dispatch None (event (Open_requested (true, Keyboard)));
  ignore (accept r None : Wire.Transaction.t);
  dispatch None toggle;
  print_endline
    "current callbacks, FIFO intents, disabled and query-remount fences passed";
  [%expect
    {| current callbacks, FIFO intents, disabled and query-remount fences passed |}]
;;

let%expect_test "picker slots are unstyled keyed wrappers and reject passive interactions"
  =
  let cfg = config () in
  let picker = view cfg |> View.Expert.describe in
  List.iter picker.children ~f:(fun child ->
    let wrapper = View.Expert.describe child in
    assert (View.Expert.Kind.equal wrapper.kind Container);
    assert (Option.is_none wrapper.on_click);
    assert (List.length wrapper.children = 1);
    assert (List.is_empty (Style.Expert.to_wire wrapper.style ~theme:Theme.default |> ok)));
  let bad =
    P.Description.create
      ~config:cfg
      ~query:(P.Query.create ~controller:(key "query") () |> ok)
      ~trigger:(View.button ~on_click:(fun () -> "bad") "Bad")
      ()
    |> ok
  in
  assert (Result.is_error (View.choice_picker ~on_event:(observe "bad") bad));
  let too_many =
    P.Description.create
      ~config:cfg
      ~query:(P.Query.create ~controller:(key "query") () |> ok)
      ~footer:(View.column (List.init 4096 ~f:(fun _ -> View.text "x")))
      ()
    |> ok
  in
  assert (Result.is_error (View.choice_picker ~on_event:(observe "bad") too_many));
  let long_id = String.make 256 'x' in
  let long_item = Choice.create ~id:(choice_id long_id) ~label:"Long ID" () |> ok in
  let long_group =
    P.Group.create
      ~id:(P.Group.Id.of_string long_id |> ok)
      ~label:"Long group"
      (Choice.Collection.create [ long_item ] |> ok)
    |> ok
  in
  let long_config =
    P.Config.create
      ~label:"Long IDs"
      ~options:(P.Collection.grouped [ long_group ] |> ok)
      ~selected:(P.Selection.single None)
      ()
    |> ok
  in
  let long_description =
    P.Description.create
      ~config:long_config
      ~groups:[ P.Group.id long_group, View.text "Group" ]
      ~options:[ choice_id long_id, P.Option_content.create (View.text "Item") ]
      ()
    |> ok
  in
  let long_view = View.choice_picker ~on_event:(observe "long") long_description |> ok in
  ignore (accept (Reconciler.create window) (Some long_view) : Wire.Transaction.t);
  let r = Reconciler.create window in
  assert (
    Result.is_error
      (Reconciler.prepare
         r
         ~theme:Theme.default
         (Some
            (View.column
               [ View.with_key (view cfg) (key "left")
               ; View.with_key (view cfg) (key "right")
               ]))));
  print_endline "slot shape, passive input and aggregate child/controller budgets checked";
  [%expect {| slot shape, passive input and aggregate child/controller budgets checked |}]
;;
