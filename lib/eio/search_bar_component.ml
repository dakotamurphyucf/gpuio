open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Input = Gpuio.Text_input
module S = Input.Search
module Rows = Gpuio_bonsai.Managed_rows

let ok = Or_error.ok_exn
let key = Gpuio.Key.of_string_exn
let style = Gpuio.Style.create_exn
let px = Gpuio.Length.px_exn

type target =
  { search : S.Snapshot.t option
  ; command : S.Command.t -> (S.Response.t, Input.Command_error.t) Result.t E.t
  }

module Actions = struct
  type t =
    { run : S.Command.t -> unit E.t
    ; navigate : S.Command.t -> unit E.t
    ; close : unit E.t
    ; replace : bool -> unit E.t
    ; toggle_case : unit E.t
    ; reopen : replace:bool -> unit E.t
    ; invalid_query : unit E.t
    }
end

type session =
  { reopen : replace:bool -> unit E.t
  ; view : Gpuio.Style.t option -> V.t
  ; commands : unit E.t Gpuio.Command.t list
  }

type t =
  { open_search : replace:bool -> unit E.t
  ; session : session option
  ; error : string option
  }

let error_message = function
  | Input.Command_error.Composing -> "Finish composing text before using this action."
  | Stale_search | Stale_revision ->
    "Search changed. Review the current match and try again."
  | Limit_exceeded -> "The replacement would exceed the editor's size limit."
  | Invalid_text -> "This text is not allowed by the editor's input rules."
  | Not_editable -> "This editor cannot currently be edited."
  | Not_mounted | Stale_editor | Closed | Search_unavailable ->
    "Search is no longer available for this editor."
  | Focus_blocked -> "This editor is currently hidden or unavailable."
  | Invalid_selection | Busy | Native_failure ->
    "The search action could not complete. Please try again."
;;

let chord ?(modifiers = []) name =
  Gpuio.Shortcut.create ~key:name ~modifiers ~priority:Override ~text_input:Always ()
  |> ok
;;

let command name label shortcuts action =
  Gpuio.Command.create
    ~id:(Gpuio.Command.Id.of_string ("gpuio.search." ^ name) |> ok)
    ~label
    ~shortcuts
    ~on_invoke:(fun () -> action)
    ()
  |> ok
;;

let status ?(alert = false) message =
  V.with_accessibility
    (V.text message)
    (Gpuio.Accessibility.create ~role:(if alert then Alert else Status) () |> ok)
  |> ok
;;

let field query ~initial_text ~label ~auto_focus =
  let config =
    Input.Config.create
      ~mode:Multiline
      ~label
      ~placeholder:label
      ~auto_focus
      ~min_rows:1
      ~max_rows:3
      ()
    |> ok
  in
  Gpuio.View.text_input
    ~controller:(Editor_controller.key query)
    ~config
    ~initial_text
    ~style:(style [ Grow 1.; Min_width (px 100.) ])
    ~on_event:(function
      | Input.Event.Changed snapshot -> Editor_controller.observe query snapshot
      | Submitted submission ->
        Editor_controller.observe query (Input.Expert.submission_snapshot submission)
      | Search_changed _ -> E.Ignore)
    ()
  |> ok
;;

let session edit ~target ~replacement_seed ~save_replacement lifetime graph =
  let query = Editor_controller.create_with_command edit graph in
  let replacement = Editor_controller.create_with_command edit graph in
  let error, set_error = B.state_opt ~equal:String.equal graph in
  let notice_epoch = B.Expert.thunk ~f:(fun () -> ref 0) graph in
  let open B.Let_syntax in
  let actions =
    let%arr target = target
    and query = query
    and replacement = replacement
    and lifetime = lifetime
    and set_error = set_error
    and notice_epoch = notice_epoch in
    let guard = Rows.Lifetime.guard lifetime in
    (* One notice epoch per user workflow, not per transport stage. A query
       echo must not cancel a pending Next continuation; only stale feedback is
       suppressed. Native stamps still guard every destructive operation. *)
    let perform f =
      guard
        (E.bind
           (E.of_thunk (fun () ->
              Int.incr notice_epoch;
              !notice_epoch))
           ~f:(fun epoch ->
             let notice message =
               guard
                 (E.bind
                    (E.of_thunk (fun () -> Int.equal epoch !notice_epoch))
                    ~f:(fun current -> if current then set_error message else E.Ignore))
             in
             let report error = notice (Some (error_message error)) in
             let send command continue =
               guard
                 (E.bind (target.command command) ~f:(function
                    | Error error -> report error
                    | Ok _ -> guard (E.Many [ notice None; continue ])))
             in
             let read editor continue =
               guard
                 (E.bind (Editor_controller.command editor Read_snapshot) ~f:(function
                    | Error error -> report error
                    | Ok snapshot
                      when Option.is_some (Input.Snapshot.composition snapshot) ->
                      report Composing
                    | Ok snapshot -> guard (continue snapshot)))
             in
             f ~notice ~send ~read))
    in
    let invalid_message = Some "Search queries are limited to 2,048 UTF-8 bytes." in
    let run command = perform (fun ~notice:_ ~send ~read:_ -> send command E.Ignore) in
    let invalid_query = perform (fun ~notice ~send:_ ~read:_ -> notice invalid_message) in
    let metadata = Option.value_exn target.search in
    let navigate direction =
      perform (fun ~notice ~send ~read ->
        read query (fun snapshot ->
          match S.Query.create (Input.Snapshot.text snapshot) with
          | Error _ -> notice invalid_message
          | Ok query -> send (Set_query_text query) (send direction E.Ignore)))
    in
    let close =
      perform (fun ~notice:_ ~send ~read ->
        read query (fun _ ->
          read replacement (fun _ -> send (Close_and_focus metadata) E.Ignore)))
    in
    let replace all =
      perform (fun ~notice ~send ~read ->
        read query (fun query ->
          if
            not
              (String.equal
                 (Input.Snapshot.text query)
                 (S.Query.to_string (S.Snapshot.query metadata)))
          then notice (Some "Search is updating. Review the matches and try again.")
          else
            read replacement (fun replacement ->
              let replacement = Input.Snapshot.text replacement in
              let if_stamp = S.Snapshot.stamp metadata in
              send
                (if all
                 then Replace_all { if_stamp; replacement }
                 else Replace_current { if_stamp; replacement })
                E.Ignore)))
    in
    let reopen ~replace =
      perform (fun ~notice:_ ~send ~read ->
        read query (fun _ -> read replacement (fun _ -> send (Open { replace }) E.Ignore)))
    in
    let toggle_case =
      perform (fun ~notice:_ ~send ~read ->
        read query (fun _ -> send Toggle_case E.Ignore))
    in
    { Actions.run; navigate; close; replace; toggle_case; reopen; invalid_query }
  in
  (* Select the initial query only if it is still the exact unchanged mount.
     Native auto_focus handles visibility/placement, avoiding an early Focus RPC. *)
  B.Edge.on_change
    (B.map query ~f:(fun query ->
       Option.map (Editor_controller.snapshot query) ~f:Input.Expert.node))
    ~equal:(Option.equal Gpuio_protocol.Node_id.equal)
    ~callback:
      (let%arr query = query
       and target = target
       and lifetime = lifetime in
       fun _ ->
         match Editor_controller.snapshot query with
         | None -> E.Ignore
         | Some expected
           when (not
                   (Int64.equal
                      (Input.Revision.to_int64 (Input.Snapshot.revision expected))
                      0L))
                || Option.is_some (Input.Snapshot.composition expected)
                || not
                     (String.equal
                        (Input.Snapshot.text expected)
                        (S.Query.to_string
                           (S.Snapshot.query (Option.value_exn target.search)))) ->
           E.Ignore
         | Some expected ->
           let text = Input.Snapshot.text expected in
           let selection =
             Input.Selection.create ~anchor:0 ~head:(String.length text) |> ok
           in
           Rows.Lifetime.guard
             lifetime
             (E.map
                (Editor_controller.replace_if_unchanged
                   query
                   expected
                   ~selection:(Select selection)
                   ~undo:Reset
                   text)
                ~f:(fun _ -> ())))
    graph;
  B.Edge.on_change
    (B.map query ~f:(fun query ->
       Option.map (Editor_controller.snapshot query) ~f:(fun s ->
         Input.Snapshot.text s, Option.is_some (Input.Snapshot.composition s))))
    ~equal:(Option.equal [%equal: string * bool])
    ~callback:
      (let%arr actions = actions in
       fun observed ->
         match observed with
         | None | Some (_, true) -> E.Ignore
         | Some (text, false) ->
           (* Even returning to the last observed native query must be sent: an
              earlier different query may still be in flight. Identical native
              echoes preserve the session revision and match position. *)
           (match S.Query.create text with
            | Ok query -> actions.run (Set_query_text query)
            | Error _ -> actions.invalid_query))
    graph;
  B.Edge.on_change
    (B.map replacement ~f:(fun input ->
       Option.bind (Editor_controller.snapshot input) ~f:(fun s ->
         Option.some_if
           (Option.is_none (Input.Snapshot.composition s))
           (Input.Snapshot.text s))))
    ~equal:(Option.equal String.equal)
    ~callback:
      (let%arr save = save_replacement
       and lifetime = lifetime in
       fun value ->
         Option.value_map value ~default:E.Ignore ~f:(fun value ->
           Rows.Lifetime.guard lifetime (save value)))
    graph;
  let%arr target = target
  and query = query
  and replacement = replacement
  and replacement_seed = replacement_seed
  and error = error
  and actions = actions in
  let metadata = Option.value_exn target.search in
  let { Actions.run = _
      ; navigate
      ; close
      ; replace
      ; toggle_case
      ; reopen
      ; invalid_query = _
      }
    =
    actions
  in
  let next = navigate S.Command.Next
  and previous = navigate S.Command.Previous in
  let mode = S.Snapshot.mode metadata in
  let match_case = S.Case.equal (S.Snapshot.case metadata) Sensitive in
  let can_replace =
    S.Snapshot.can_replace metadata && S.Snapshot.match_count metadata > 0
  in
  let button ?(disabled = false) label action =
    V.button
      ~disabled
      ~config:(Gpuio.Button.Config.create ~focus:Preserve ())
      ~on_click:action
      label
  in
  let query_view =
    V.command_scope
      ~style:(style [ Grow 1.; Min_width (px 160.) ])
      ~commands:
        (Gpuio.Command.Registry.create
           [ command "query-next" "Next match" [ chord "enter" ] next
           ; command
               "query-previous"
               "Previous match"
               [ chord ~modifiers:[ Shift ] "enter" ]
               previous
           ]
         |> ok)
      [ field
          query
          ~label:"Find in editor"
          ~initial_text:(S.Query.to_string (S.Snapshot.query metadata))
          ~auto_focus:true
      ]
  in
  let count =
    match S.Snapshot.current metadata with
    | None when String.is_empty (S.Query.to_string (S.Snapshot.query metadata)) ->
      "Type to search"
    | None -> "No matches"
    | Some occurrence ->
      sprintf
        "%d of %d"
        (S.Occurrence.index occurrence + 1)
        (S.Snapshot.match_count metadata)
  in
  let view custom_style =
    V.column
      ~style:
        (Gpuio.Style.merge
           [ style [ Gap (px 8.); Padding (px 8.) ]
           ; Option.value custom_style ~default:Gpuio.Style.empty
           ])
      ([ V.row
           ~style:(style [ Gap (px 8.); Wrap Wrap ])
           [ query_view
           ; button "Previous" previous
           ; button "Next" next
           ; button "Close search" close
           ]
       ; V.row
           ~style:(style [ Gap (px 12.); Wrap Wrap ])
           [ V.checkbox
               ~state:(if match_case then Checked else Unchecked)
               ~on_toggle:toggle_case
               "Match case"
           ; button
               ~disabled:(not (S.Snapshot.can_replace metadata))
               (if S.Mode.equal mode Replace
                then "Hide replacement"
                else "Show replacement")
               (reopen ~replace:(not (S.Mode.equal mode Replace)))
           ; status count
           ]
       ; V.column
           ~key:(key "replacement")
           ~style:(style [ Display (if S.Mode.equal mode Replace then Flex else Hidden) ])
           [ V.row
               ~style:(style [ Gap (px 8.); Wrap Wrap ])
               [ field
                   replacement
                   ~label:"Replace with"
                   ~initial_text:replacement_seed
                   ~auto_focus:false
               ; button ~disabled:(not can_replace) "Replace" (replace false)
               ; button ~disabled:(not can_replace) "Replace all" (replace true)
               ]
           ]
       ]
       @ Option.to_list (Option.map error ~f:(status ~alert:true)))
  in
  { reopen
  ; view
  ; commands =
      [ command "next" "Next match" [ chord "f3" ] next
      ; command "previous" "Previous match" [ chord ~modifiers:[ Shift ] "f3" ] previous
      ; command "close" "Close search" [ chord "escape" ] close
      ]
  }
;;

let create edit ~target graph =
  let replacement_seed, save_replacement = B.state "" graph in
  let error, set_error = B.state_opt ~equal:String.equal graph in
  let open_epoch = B.Expert.thunk ~f:(fun () -> ref 0) graph in
  B.Edge.lifecycle
    ~on_deactivate:
      (B.map open_epoch ~f:(fun epoch -> E.of_thunk (fun () -> Int.incr epoch)))
    graph;
  let openings =
    B.map target ~f:(fun target ->
      match target.search with
      | None -> String.Map.empty
      | Some search when S.Mode.equal (S.Snapshot.mode search) Closed -> String.Map.empty
      | Some search ->
        let stamp = S.Snapshot.stamp search in
        let identity =
          [%sexp
            (S.Expert.stamp_window stamp : Gpuio_protocol.Window_id.t)
          , (S.Expert.stamp_node stamp : Gpuio_protocol.Node_id.t)
          , (S.Snapshot.activation_revision search : int64)]
          |> Sexp.to_string_mach
        in
        String.Map.singleton identity target)
  in
  let sessions =
    Rows.assoc
      (module String)
      openings
      ~f:(fun _ target lifetime graph ->
        session edit ~target ~replacement_seed ~save_replacement lifetime graph)
      graph
  in
  let open B.Let_syntax in
  let%arr target = target
  and sessions = sessions
  and error = error
  and set_error = set_error
  and open_epoch = open_epoch in
  let session = Map.min_elt sessions |> Option.map ~f:snd in
  { open_search =
      (fun ~replace ->
        E.bind
          (E.of_thunk (fun () ->
             Int.incr open_epoch;
             !open_epoch))
          ~f:(fun epoch ->
            match session with
            | Some session -> E.Many [ set_error None; session.reopen ~replace ]
            | None ->
              E.bind
                (target.command (Open { replace }))
                ~f:(fun result ->
                  E.bind
                    (E.of_thunk (fun () -> Int.equal epoch !open_epoch))
                    ~f:(fun current ->
                      if not current
                      then E.Ignore
                      else (
                        match result with
                        | Ok _ -> set_error None
                        | Error error -> set_error (Some (error_message error)))))))
  ; session
  ; error
  }
;;

let open_ t ?(replace = false) () = t.open_search ~replace

let wrap ?style:outer_style ?bar_style t content =
  let commands =
    [ command "find" "Find in editor" [ chord ~modifiers:[ Primary ] "f" ] (open_ t ())
    ; command
        "find-replace"
        "Find and replace"
        [ chord ~modifiers:[ Primary; Shift ] "f" ]
        (open_ t ~replace:true ())
    ]
    @ Option.value_map t.session ~default:[] ~f:(fun session -> session.commands)
  in
  V.command_scope
    ?style:outer_style
    ~commands:(Gpuio.Command.Registry.create commands |> ok)
    [ V.column
        ~key:(key "search-bar")
        (Option.to_list (Option.map t.session ~f:(fun session -> session.view bar_style)))
    ; V.column ~key:(key "editor-content") [ content ]
    ; V.column
        ~key:(key "search-error")
        (Option.to_list (Option.map t.error ~f:(status ~alert:true)))
    ]
;;
