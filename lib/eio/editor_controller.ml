open Core
module Bonsai = Bonsai.Cont
module Input = Gpuio.Text_input

type observation =
  | Native of Input.Snapshot.t
  | Reply of Input.Snapshot.t * Input.Snapshot.t
  | Search_native of Input.Search.Snapshot.t
  | Search_reply of Input.Snapshot.t * Input.Search.Snapshot.t

type model =
  { text : Input.Snapshot.t option
  ; search : Input.Search.Snapshot.t option
  }
[@@deriving equal]

type command =
  Input.Snapshot.t
  -> Input.Command.t
  -> (Input.Snapshot.t, Input.Command_error.t) Result.t Bonsai.Effect.t

type t =
  { command : command
  ; controller : Gpuio.Key.t
  ; snapshot : Input.Snapshot.t option
  ; search_snapshot : Input.Search.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (Input.Expert.window left) (Input.Expert.window right)
  && Gpuio_protocol.Node_id.equal (Input.Expert.node left) (Input.Expert.node right)
;;

let search_matches_editor search editor =
  let stamp = Input.Search.Snapshot.stamp search in
  Gpuio_protocol.Window_id.equal
    (Input.Search.Expert.stamp_window stamp)
    (Input.Expert.window editor)
  && Gpuio_protocol.Node_id.equal
       (Input.Search.Expert.stamp_node stamp)
       (Input.Expert.node editor)
;;

let install_search previous next =
  let stamp search =
    Input.Search.Expert.stamp_to_wire (Input.Search.Snapshot.stamp search)
  in
  let next_stamp = stamp next in
  let current =
    Option.exists previous.text ~f:(fun text ->
      search_matches_editor next text
      && Int64.(
           next_stamp.editor_revision
           >= Input.Revision.to_int64 (Input.Snapshot.revision text)))
  in
  let newer =
    Option.for_all previous.search ~f:(fun search ->
      let old = stamp search in
      Int64.(
        next_stamp.editor_revision >= old.editor_revision
        && next_stamp.search_revision >= old.search_revision))
  in
  if current && newer then { previous with search = Some next } else previous
;;

let install_text previous next =
  match previous.text with
  | Some old
    when same_lease old next
         && Input.Revision.compare
              (Input.Snapshot.revision old)
              (Input.Snapshot.revision next)
            > 0 -> previous
  | old ->
    { text = Some next
    ; search =
        (if Option.exists old ~f:(fun old -> same_lease old next)
         then previous.search
         else None)
    }
;;

let apply_observation previous = function
  | Search_native next -> install_search previous next
  | Search_reply (expected, next) ->
    (* Read-only/composition capability observations can share a stamp. A delayed
       reply must not undo that native observation; existing metadata wins ties. *)
    let repeats_stamp =
      Option.exists previous.search ~f:(fun current ->
        Input.Search.Stamp.equal
          (Input.Search.Snapshot.stamp current)
          (Input.Search.Snapshot.stamp next))
    in
    if
      (not repeats_stamp)
      && Option.exists previous.text ~f:(fun text -> same_lease text expected)
    then install_search previous next
    else previous
  | Native next -> install_text previous next
  | Reply (expected, next) ->
    if Option.exists previous.text ~f:(fun text -> same_lease text expected)
    then install_text previous next
    else previous
;;

let create_with_command command graph =
  let model, observe =
    Bonsai.state_machine0
      ~default_model:{ text = None; search = None }
      ~equal:equal_model
      ~apply_action:(fun _ previous observation -> apply_observation previous observation)
      graph
  in
  let controller = Bonsai.path_id graph in
  let open Bonsai.Let_syntax in
  let%arr model = model
  and observe = observe
  and controller = controller in
  { command
  ; controller = Gpuio.Key.of_string controller |> Or_error.ok_exn
  ; snapshot = model.text
  ; search_snapshot = model.search
  ; observe
  }
;;

let create window = create_with_command (App.Window.Expert.editor_command window)
let key t = t.controller
let observe t snapshot = t.observe (Native snapshot)
let observe_reply t ~expected snapshot = t.observe (Reply (expected, snapshot))
let snapshot t = t.snapshot
let search_snapshot t = t.search_snapshot
let observe_search t snapshot = t.observe (Search_native snapshot)

let observe_search_reply t ~expected snapshot =
  t.observe (Search_reply (expected, snapshot))
;;

let send t snapshot command =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = t.command snapshot command in
  let%map () =
    match result with
    | Ok next -> t.observe (Reply (snapshot, next))
    | Error _ -> Bonsai.Effect.Ignore
  in
  result
;;

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot -> send t snapshot command
;;

let focus t = command t Focus
let select t selection = command t (Select selection)

let replace t ?if_revision ~selection ~undo text =
  command t (Replace { text; selection; undo; if_revision })
;;

let replace_if_unchanged t expected ~selection ~undo text =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some current when not (same_lease current expected) ->
    Bonsai.Effect.return (Error Input.Command_error.Stale_editor)
  | Some _ ->
    send
      t
      expected
      (Replace
         { text; selection; undo; if_revision = Some (Input.Snapshot.revision expected) })
;;

let%expect_test "search metadata cannot replace text or cross an editor lease" =
  let ok = Or_error.ok_exn in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node generation = Gpuio_protocol.Node_id.create ~slot:1L ~generation |> ok in
  let text generation revision value =
    Input.Expert.snapshot_of_wire
      ~window
      ~node:(node generation)
      { revision
      ; text = value
      ; selection = { anchor = 0L; head = 0L }
      ; composition = None
      ; focused = true
      }
    |> ok
  in
  let search generation editor_revision search_revision can_replace =
    Input.Search.Expert.snapshot_of_wire
      ~window
      ~node:(node generation)
      { stamp = { editor_revision; search_revision }
      ; activation_revision = 1L
      ; mode = Find
      ; query = ""
      ; case = Sensitive
      ; text_bytes = 3L
      ; match_count = 0L
      ; current = None
      ; can_replace
      }
    |> ok
  in
  let initial = { text = None; search = None } in
  let old_text = text 1L 2L "old" in
  let initial = apply_observation initial (Native old_text) in
  let first = search 1L 2L 3L true in
  let observed = apply_observation initial (Search_native first) in
  assert (Option.equal Input.Snapshot.equal observed.text (Some old_text));
  assert (Option.equal Input.Search.Snapshot.equal observed.search (Some first));
  let readonly = search 1L 2L 3L false in
  let observed = apply_observation observed (Search_native readonly) in
  assert (Option.equal Input.Search.Snapshot.equal observed.search (Some readonly));
  assert (
    equal_model (apply_observation observed (Search_reply (old_text, first))) observed);
  let typed = text 1L 4L "new" in
  let observed = apply_observation observed (Native typed) in
  assert (Option.is_some observed.search);
  List.iter
    [ Search_reply (old_text, first)
    ; Search_native first
    ; Search_native (search 1L 4L 2L true)
    ]
    ~f:(fun action -> assert (equal_model (apply_observation observed action) observed));
  let newest = search 1L 4L 5L true in
  let observed = apply_observation observed (Search_reply (old_text, newest)) in
  assert (Option.equal Input.Search.Snapshot.equal observed.search (Some newest));
  let mounted = apply_observation observed (Native (text 2L 0L "new")) in
  assert (Option.is_none mounted.search);
  List.iter
    [ Search_native newest
    ; Search_reply (old_text, newest)
    ; Reply (old_text, text 1L 10L "bad")
    ]
    ~f:(fun action -> assert (equal_model (apply_observation mounted action) mounted));
  let current = search 2L 0L 1L true in
  assert (
    Option.equal
      Input.Search.Snapshot.equal
      (apply_observation mounted (Search_native current)).search
      (Some current));
  print_endline "text preserved; late metadata rejected; replacement mount clears search";
  [%expect {| text preserved; late metadata rejected; replacement mount clears search |}]
;;
