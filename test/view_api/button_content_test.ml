open Core
open Gpuio
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let key name = Key.of_string_exn name

let commit r view =
  let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let rich ?config content =
  View.button_with_content
    ~key:(key "button")
    ?config
    ~accessible_name:"Upload"
    ~on_click:(fun () -> "upload")
    content
;;

let%expect_test
    "rich button content is checked and ordinary handlers retire during loading"
  =
  let config =
    Progress.Config.create ~label:"Uploading" ~value:Progress.Value.indeterminate |> ok
  in
  let content = View.progress ~config () in
  assert (Result.is_ok (rich content));
  assert (Result.is_error (rich (View.button ~on_click:(fun () -> "nested") "Nested")));
  let bindings = Command_binding.Config.create [ Native_action Copy ] |> ok in
  assert (
    Result.is_error
      (rich
         (View.command_binding_scope
            ~config:bindings
            ~on_update:(fun _ -> "observed")
            [ View.text "binding label" ])));
  assert (
    Result.is_error
      (rich (View.text ~style:(Style.create_exn [ User_select true ]) "Selectable")));
  List.iter
    [ ""; " \t"; "bad\000name"; "\255"; String.make 1025 'a' ]
    ~f:(fun accessible_name ->
      assert (
        Result.is_error
          (View.button_with_content ~accessible_name ~on_click:Fn.id (View.text "label"))));
  let r = Reconciler.create window in
  let operations = commit r (rich content |> ok) in
  let node, handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let press = Wire.Event.Press (window, node, handler, Reconciler.revision r) in
  assert (Option.equal String.equal (Reconciler.dispatch r press) (Some "upload"));
  let loading = Button.Config.create ~loading:true () in
  let operations = commit r (rich ~config:loading content |> ok) in
  assert (
    List.exists operations ~f:(function
      | Wire.Op.Bind (_, None) -> true
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch r press));
  ignore (commit r (rich content |> ok) : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r press));
  let operations =
    commit r (View.button ~key:(key "button") ~on_click:(fun () -> "plain") "Upload")
  in
  assert (
    List.exists operations ~f:(function
      | Wire.Op.Set_button_presentation (owner, None) ->
        Gpuio_protocol.Node_id.equal owner node
      | _ -> false));
  assert (
    not
      (List.exists operations ~f:(function
         | Wire.Op.Create (_, Button, _, _) -> true
         | _ -> false)));
  Reconciler.close r;
  assert (Option.is_none (Reconciler.dispatch r press));
  [%expect {| |}]
;;

let%expect_test
    "queued command clicks obey each current button through loading and ref changes"
  =
  let run = Command.Id.of_string "run" |> ok in
  let other = Command.Id.of_string "other" |> ok in
  let commands =
    Command.Registry.create
      [ Command.create ~id:run ~label:"Run" ~on_invoke:(fun () -> "run") () |> ok
      ; Command.create ~id:other ~label:"Other" ~on_invoke:(fun () -> "other") () |> ok
      ]
    |> ok
  in
  let fixed = View.command_button ~key:(key "fixed") ~command:run () in
  let view ?(command = run) ?(include_busy = true) ?(loading = false) () =
    let busy =
      View.command_button_with_content
        ~key:(key "busy")
        ~config:(Button.Config.create ~loading ())
        ~command
        (View.text "Rich")
      |> ok
    in
    View.command_scope ~commands (if include_busy then [ busy; fixed ] else [ fixed ])
  in
  let r = Reconciler.create window in
  let operations = commit r (view ()) in
  let scope, handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Create (node, Command_scope, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let buttons =
    List.filter_map operations ~f:(function
      | Wire.Op.Create (node, Command_button, _, _) -> Some node
      | _ -> None)
  in
  let busy, fixed =
    match buttons with
    | [ a; b ] -> a, b
    | _ -> assert false
  in
  let generation =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Set_commands (_, commands) ->
        List.find_map commands ~f:(fun command ->
          if String.equal command.id "run" then Some command.generation else None)
      | _ -> None)
  in
  let event ?(revision = 1L) source =
    Wire.Event.Command_invoked
      (window, scope, handler, revision, "run", generation, source)
  in
  let invoke ?revision source = Reconciler.dispatch r (event ?revision source) in
  assert (Option.equal String.equal (invoke (Button busy)) (Some "run"));
  let prepared =
    Reconciler.prepare r ~theme:Theme.default (Some (view ~loading:true ())) |> ok
  in
  (* Preparation does not change accepted callback policy. *)
  assert (Option.is_some (invoke (Button busy)));
  Reconciler.accept r prepared |> ok;
  assert (Option.is_none (invoke (Button busy)));
  assert (Option.is_some (invoke (Button fixed)));
  assert (Option.is_some (invoke Shortcut));
  ignore (commit r (view ()) : Wire.Op.t list);
  assert (Option.is_none (invoke (Button busy)));
  assert (Option.is_some (invoke ~revision:(Reconciler.revision r) (Button busy)));
  assert (Option.is_some (invoke (Button fixed)));
  let before_ref_change = Reconciler.revision r in
  ignore (commit r (view ~command:other ()) : Wire.Op.t list);
  assert (Option.is_none (invoke ~revision:(Reconciler.revision r) (Button busy)));
  ignore (commit r (view ()) : Wire.Op.t list);
  assert (Option.is_none (invoke ~revision:before_ref_change (Button busy)));
  assert (Option.is_some (invoke ~revision:(Reconciler.revision r) (Button busy)));
  ignore (commit r (view ~include_busy:false ()) : Wire.Op.t list);
  assert (Option.is_none (invoke (Button busy)));
  ignore (commit r (view ()) : Wire.Op.t list);
  assert (Option.is_none (invoke ~revision:(Reconciler.revision r) (Button busy)));
  Reconciler.close r;
  assert (Option.is_none (invoke Shortcut));
  [%expect {| |}]
;;
