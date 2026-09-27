open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Loader = Gpuio_eio.Tree_loading
module Tree = Gpuio_bonsai.Tree
module T = Gpuio.Tree
module I = Gpuio.Tree_interaction
module View = Gpuio_bonsai.View
module Data = Tree_example_model.Outline_data

let ok = Or_error.ok_exn
let style = Gpuio.Style.create_exn
let px = Gpuio.Length.px_exn

(* Application-owned approval state, updated only by UI effects. A token makes
   queued confirmations one-shot and prevents an older dialog approving a newer
   proposal. It holds no snapshot; approval reads the current loader data. *)
module Decision = struct
  type pending =
    { token : int
    ; proposal : I.Move.t
    }

  type t =
    { serial : int
    ; pending : pending option
    ; notice : string
    ; approved : int
    }

  let initial =
    { serial = 0
    ; pending = None
    ; notice = "Drag an item, or use its Actions menu."
    ; approved = 0
    }
  ;;
end

module Model = struct
  type t =
    { loader : string Loader.t
    ; decision : Decision.t B.Expert.Var.t
    }

  let update t ~f = B.Expert.Var.set t.decision (f (B.Expert.Var.get t.decision))

  let propose t proposal =
    update t ~f:(fun previous ->
      if previous.serial = Int.max_value then failwith "example approval serial exhausted";
      let serial = previous.serial + 1 in
      { previous with
        serial
      ; pending = Some { token = serial; proposal }
      ; notice = "Review the proposed move."
      })
  ;;

  let cancel t token =
    update t ~f:(fun previous ->
      if Option.exists previous.pending ~f:(fun pending -> pending.token = token)
      then { previous with pending = None; notice = "Move cancelled." }
      else previous)
  ;;

  let approve t token state =
    let previous = B.Expert.Var.get t.decision in
    match previous.pending with
    | None -> ()
    | Some pending when pending.token <> token -> ()
    | Some pending ->
      (* Consume before invoking application mutation; duplicate callbacks cannot
         approve twice, even before the next Bonsai/native frame. *)
      B.Expert.Var.set t.decision { previous with pending = None };
      let result =
        Or_error.bind
          (Data.approve (Loader.snapshot t.loader) ~state pending.proposal)
          ~f:(Loader.update t.loader)
      in
      update t ~f:(fun current ->
        match result with
        | Ok () ->
          { current with
            approved = current.approved + 1
          ; notice = "Move applied. Stable item identity preserved."
          }
        | Error error -> { current with notice = Error.to_string_hum error })
  ;;
end

module Observed = struct
  type t =
    { output : string Tree.Output.t
    ; view : View.t
    ; confirm : unit E.t
    ; decision : Decision.t
    }
end

let config = Gpuio.Virtual_list.Config.create ~height:(Fixed 42.) ~max_active:16 () |> ok
let command_id = Gpuio.Command.Id.of_string
let menu_id name = command_id name |> ok
let scope_key item = Gpuio.Key.of_string_exn ("actions:" ^ T.Id.to_string item)

let render_item source ~target ~item ~controller ~lifetime _graph =
  let open B.Let_syntax in
  let%arr source = source
  and target = target
  and item = item
  and controller = controller
  and lifetime = lifetime in
  let guard = Gpuio_bonsai.Managed_rows.Lifetime.guard lifetime in
  let move destination placement =
    match I.Target.capture source (Data.id destination) with
    | Error _ -> E.Ignore
    | Ok destination ->
      guard
        (E.Many
           [ Tree.Controller.reveal controller destination
           ; Tree.Controller.propose_move controller ~source:target ~destination placement
           ])
  in
  let commands =
    Gpuio.Command.Registry.create
      [ Gpuio.Command.create
          ~id:(menu_id "open")
          ~label:"Open item"
          ~on_invoke:(fun () -> guard (Tree.Controller.activate controller target))
          ()
        |> ok
      ; Gpuio.Command.create
          ~id:(menu_id "inbox")
          ~label:"Move to Inbox"
          ~enabled:(not (T.Id.equal item.Gpuio.Tree_rows.Item.id (Data.id "inbox")))
          ~on_invoke:(fun () -> move "inbox" Inside)
          ()
        |> ok
      ; Gpuio.Command.create
          ~id:(menu_id "archive")
          ~label:"Move to Archive"
          ~enabled:(not (T.Id.equal item.id (Data.id "archive")))
          ~on_invoke:(fun () -> move "archive" Inside)
          ()
        |> ok
      ; Gpuio.Command.create
          ~id:(menu_id "top")
          ~label:"Move before Inbox"
          ~enabled:(not (T.Id.equal item.id (Data.id "inbox")))
          ~on_invoke:(fun () -> move "inbox" Before)
          ()
        |> ok
      ]
    |> ok
  in
  let menu =
    Gpuio.Menu.create
      ~label:"Actions"
      [ Command (menu_id "open")
      ; Separator
      ; Command (menu_id "inbox")
      ; Command (menu_id "archive")
      ; Command (menu_id "top")
      ]
    |> ok
  in
  View.command_scope
    ~key:(scope_key item.id)
    ~commands
    ~style:(style [ Grow 1.; Min_width (px 0.) ])
    [ View.context_menu
        ~menu
        (View.row
           ~style:(style [ Grow 1.; Min_width (px 0.); Align_items Center; Gap (px 12.) ])
           [ View.text
               ~style:
                 (style
                    [ Grow 1.
                    ; Min_width (px 0.)
                    ; White_space No_wrap
                    ; Text_overflow Ellipsis
                    ])
               (T.Node.label item.node)
           ; View.menu_button ~menu ()
           ])
    ]
;;

let component model observed _window graph =
  let open B.Let_syntax in
  let source = Loader.value model.Model.loader in
  let decision = B.Expert.Var.value model.decision in
  let on_action =
    B.return (function
      | I.Action.None -> E.Ignore
      | Move proposal -> E.of_thunk (fun () -> Model.propose model proposal)
      | Activate id ->
        E.of_thunk (fun () ->
          Model.update model ~f:(fun decision ->
            let notice =
              Option.value_map
                (T.find
                   (Gpuio.Tree_loading.Snapshot.tree (Loader.snapshot model.loader))
                   id)
                ~default:"Item is no longer available."
                ~f:T.Node.data
            in
            { decision with notice })))
  in
  let output =
    Tree.component
      source
      ~config
      ~label:"Project outline"
      ~style:(B.return (style [ Grow 1.; Min_height (px 0.) ]))
      ~mode:(B.return Gpuio.Tree_state.Mode.Multiple)
      ~initial_expanded:(B.return [ Data.id "inbox"; Data.id "archive" ])
      ~allow_moves:(B.return true)
      ~render_item:(render_item source)
      ~on_action
      graph
  in
  let peek = B.peek output graph in
  let result =
    let%arr output = output
    and decision = decision
    and peek = peek
    and source = source in
    let confirm =
      match decision.Decision.pending with
      | None -> E.Ignore
      | Some pending ->
        E.bind peek ~f:(function
          | B.Computation_status.Inactive | Active (Error _) -> E.Ignore
          | Active (Ok output) ->
            E.of_thunk (fun () ->
              Model.approve model pending.token (Tree.Output.state output)))
    in
    let cancel =
      match decision.pending with
      | None -> E.Ignore
      | Some pending -> E.of_thunk (fun () -> Model.cancel model pending.token)
    in
    let name target =
      let id = I.Target.id target in
      match T.find (Gpuio.Tree_loading.Snapshot.tree source) id with
      | None -> T.Id.to_string id
      | Some node -> T.Node.label node
    in
    let dialog =
      Option.map decision.pending ~f:(fun pending ->
        let proposal = pending.proposal in
        let position =
          match I.Move.placement proposal with
          | Before -> "before"
          | After -> "after"
          | Inside -> "into"
        in
        View.column
          ~style:(style [ Gap (px 16.) ])
          [ View.text ~style:(style [ Font_size 22. ]) "Review move"
          ; View.text
              (sprintf
                 "Move %s %s %s?"
                 (name (I.Move.source proposal))
                 position
                 (name (I.Move.destination proposal)))
          ; View.text "This changes the sample outline only."
          ; View.row
              ~style:(style [ Gap (px 12.) ])
              [ View.button ~on_click:cancel "Cancel move"
              ; View.button ~on_click:confirm "Confirm move"
              ]
          ])
    in
    let view =
      View.column
        ~style:
          (style
             [ Width (Gpuio.Length.percent_exn 100.)
             ; Height (Gpuio.Length.percent_exn 100.)
             ; Padding (px 24.)
             ; Gap (px 14.)
             ; Background (Gpuio.Background.solid (Gpuio.Color.token_exn "background"))
             ; Foreground (Gpuio.Color.token_exn "foreground")
             ])
        [ View.text ~style:(style [ Font_size 26.; Font_weight 650 ]) "Project outline"
        ; View.text
            ~style:(style [ Foreground (Gpuio.Color.token_exn "muted") ])
            "Native drag · context actions · application-owned approval"
        ; View.text
            "Drop near an edge to reorder, or in the middle of a folder to move inside."
        ; (match output with
           | Error error -> View.text (Error.to_string_hum error)
           | Ok output -> Tree.Output.view output)
        ; View.text decision.notice
        ; View.text
            (sprintf
               "%d approved moves · 16-row budget · no filesystem changes"
               decision.approved)
        ; View.dialog
            ?key:
              (Option.map decision.pending ~f:(fun pending ->
                 Gpuio.Key.of_string_exn (sprintf "move-%d" pending.token)))
            ~config:
              (Gpuio.Overlay.Config.create ~label:"Review outline move" ~width:480. ()
               |> ok)
            ~on_dismiss:(fun _ -> cancel)
            dialog
        ]
    in
    ( view
    , Option.map (Or_error.ok output) ~f:(fun output ->
        { Observed.output; view; confirm; decision }) )
  in
  B.Edge.after_display
    (B.map result ~f:(fun (_, value) -> E.of_thunk (fun () -> observed := value)))
    graph;
  B.map result ~f:fst
;;

let rec menu_command view item command =
  let d = Gpuio.View.Expert.describe view in
  if Option.exists d.key ~f:(Gpuio.Key.equal (scope_key (Data.id item)))
  then
    Option.bind d.commands ~f:(fun registry ->
      Option.bind
        (Gpuio.Command.Registry.find registry (menu_id command))
        ~f:Gpuio.Command.Expert.invoke)
  else List.find_map d.children ~f:(fun child -> menu_command child item command)
;;

let run ~self_test ~gesture_test =
  App.run (fun env app ->
    let scope = Gpuio_eio.Scope.child (App.scope app) ~name:"outline example" |> ok in
    let loader =
      Loader.create ~scope (Data.initial ()) ~load:(fun _ ->
        Or_error.error_string "This outline is fully loaded.")
      |> ok
    in
    let model = { Model.loader; decision = B.Expert.Var.create Decision.initial } in
    let observed = ref None in
    let window =
      App.open_window
        app
        ~title:"GPUIO — Outline Lab"
        ~width:820.
        ~height:560.
        (component model observed)
      |> ok
    in
    if self_test || gesture_test
    then
      Support.start (App.scope app) (fun () ->
        let clock = Eio.Stdenv.clock env in
        let wait stage f =
          try
            Eio.Time.with_timeout_exn clock 25. (fun () ->
              while not (f ()) do
                Eio.Time.sleep clock 0.01
              done)
          with
          | Eio.Time.Timeout -> failwithf "outline test timed out: %s" stage ()
        in
        let shown () = Option.value_exn !observed in
        let parent name =
          Option.bind
            (T.position
               (Gpuio.Tree_loading.Snapshot.tree (Loader.snapshot loader))
               (Data.id name))
            ~f:(fun p -> p.parent)
        in
        wait "mounted rows" (fun () ->
          Option.exists !observed ~f:(fun shown ->
            Tree.Output.active_rows shown.output > 0));
        if gesture_test
        then (
          wait "native drag approved" (fun () ->
            (B.Expert.Var.get model.decision).approved = 1);
          assert (Option.equal T.Id.equal (parent "plan") (Some (Data.id "archive")));
          wait "native menu approved" (fun () ->
            (B.Expert.Var.get model.decision).approved = 2);
          assert (Option.equal T.Id.equal (parent "plan") (Some (Data.id "inbox"))))
        else (
          let command =
            menu_command (shown ()).view "plan" "archive" |> Option.value_exn
          in
          Support.perform scope command;
          wait "proposal" (fun () -> Option.is_some (shown ()).decision.pending);
          assert (Option.equal T.Id.equal (parent "plan") (Some (Data.id "inbox")));
          let stable = Tree.Output.target (shown ()).output (Data.id "plan") |> ok in
          let confirmation = (shown ()).confirm in
          Support.perform scope (E.Many [ confirmation; confirmation ]);
          wait "approval" (fun () ->
            (shown ()).decision.approved = 1 && Option.is_none (shown ()).decision.pending);
          assert (I.Target.is_current stable (Loader.snapshot loader));
          assert (Option.equal T.Id.equal (parent "plan") (Some (Data.id "archive")));
          Support.perform
            scope
            (menu_command (shown ()).view "plan" "inbox" |> Option.value_exn);
          wait "second proposal" (fun () -> Option.is_some (shown ()).decision.pending);
          let previous = Option.value_exn (shown ()).decision.pending in
          let superseded = (shown ()).confirm in
          Support.perform
            scope
            (menu_command (shown ()).view "plan" "top" |> Option.value_exn);
          wait "superseding proposal" (fun () ->
            Option.exists (shown ()).decision.pending ~f:(fun pending ->
              pending.token > previous.token));
          Support.perform scope superseded;
          assert ((B.Expert.Var.get model.decision).approved = 1);
          assert (
            Option.exists (B.Expert.Var.get model.decision).pending ~f:(fun pending ->
              pending.token > previous.token));
          let stale = (shown ()).confirm in
          Support.perform
            scope
            (E.of_thunk (fun () -> Loader.reset loader (Data.initial ()) |> ok));
          Support.perform scope stale;
          wait "expired approval" (fun () ->
            Option.is_none (shown ()).decision.pending
            && String.is_prefix (shown ()).decision.notice ~prefix:"Move expired");
          assert ((shown ()).decision.approved = 1);
          assert (Option.equal T.Id.equal (parent "plan") (Some (Data.id "inbox"))));
        assert (Tree.Output.active_rows (shown ()).output <= 16);
        let promise, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await promise);
        Eio.Flow.copy_string
          (if gesture_test
           then
             "TREE_OUTLINE_GESTURE_PASS native_drag=true native_menu=true approved_moves=2\n"
           else
             "TREE_OUTLINE_PASS menu_command=true confirmation_once=true \
              stable_identity=true superseded_approval=true stale_approval=true\n")
          (Eio.Stdenv.stdout env);
        App.shutdown app))
;;
