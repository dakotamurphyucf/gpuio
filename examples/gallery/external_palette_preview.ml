open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Scope = Gpuio_eio.Scope
module Controller = Gpuio_eio.Palette_controller
module Snapshot = Command_palette.Snapshot

let ok = Or_error.ok_exn

module Candidate = struct
  type t =
    { serial : int
    ; expected : Snapshot.t
    ; labels : string list
    }

  let equal a b = Int.equal a.serial b.serial

  let commands t =
    List.mapi t.labels ~f:(fun index _ ->
      Command.Id.of_string (sprintf "remote-%d-%d" t.serial index) |> ok)
  ;;
end

let component ~search window palette graph =
  let scope =
    Preview_scope.acquire
      window
      ~name:"gallery-external-palette"
      ~create:(fun scope -> E.return (Ok scope))
      graph
  in
  let controller = Controller.create window graph in
  let opened, set_opened = B.state false graph in
  let candidate, set_candidate = B.state (None : Candidate.t option) graph in
  let status, set_status = B.state "Search your workspace" graph in
  let selection, set_selection = B.state "No search result opened" graph in
  (* The page owns its producer; native result rows never own Eio tasks. *)
  let task = ref None in
  let serial = ref 0 in
  let latest = ref None in
  let cancel () =
    Option.iter !task ~f:Scope.Task.cancel;
    task := None
  in
  let retire () =
    cancel ();
    latest := None
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr controller = controller
       and set_opened = set_opened
       and set_candidate = set_candidate in
       E.Many
         [ E.of_thunk retire
         ; Controller.reset controller
         ; set_candidate None
         ; set_opened false
         ])
    graph;
  (* GPUIO triggers this lifecycle for the accepted View transaction. Command
     definitions and their rich rows are native-mounted before publication. *)
  B.Edge.on_change
    candidate
    ~equal:(Option.equal Candidate.equal)
    ~callback:
      (let%arr controller = controller
       and set_status = set_status in
       function
       | None -> E.Ignore
       | Some candidate ->
         E.bind
           (Controller.publish_results
              controller
              ~expected:candidate.Candidate.expected
              (Command_palette.Results.create ~commands:(Candidate.commands candidate) ()
               |> ok))
           ~f:(function
             | Ok _ -> set_status "Search results ready"
             | Error Query_changed
             | Error Stale_palette
             | Error Not_mounted
             | Error Closed -> E.Ignore
             | Error error ->
               set_status
                 (Sexp.to_string_hum (Command_palette.Command_error.sexp_of_t error))))
    graph;
  let%arr p = palette
  and scope = scope
  and controller = controller
  and opened = opened
  and set_opened = set_opened
  and candidate = candidate
  and set_candidate = set_candidate
  and status = status
  and set_status = set_status
  and selection = selection
  and set_selection = set_selection in
  let close =
    E.Many
      [ E.of_thunk retire
      ; Controller.reset controller
      ; set_candidate None
      ; set_opened false
      ]
  in
  let observe expected =
    let changed = not (Option.exists !latest ~f:(Snapshot.same_query expected)) in
    latest := Some expected;
    let launch =
      if not changed
      then E.Ignore
      else (
        cancel ();
        incr serial;
        let request = !serial in
        let current () =
          Int.equal request !serial
          && Option.exists !latest ~f:(Snapshot.same_query expected)
        in
        if Snapshot.composing expected
        then E.Many [ set_candidate None; set_status "Finish composing to search" ]
        else (
          match scope with
          | Preview_scope.Loading -> E.Ignore
          | Failed error -> set_status (Error.to_string_hum error)
          | Ready scope ->
            let open E.Let_syntax in
            let%bind () = set_candidate None in
            let%bind () = set_status "Searching workspace…" in
            let%bind (_ : (Snapshot.t, Command_palette.Command_error.t) Result.t) =
              Gpuio_eio.App.Window.Expert.palette_command
                window
                expected
                ~if_query_unchanged:true
                (Set_loading true)
            in
            let%bind started =
              E.of_thunk (fun () ->
                if not (current ())
                then Ok None
                else
                  Scope.start
                    scope
                    ~f:(fun () -> search (Snapshot.query expected))
                    ~on_result:(fun result ->
                      if not (current ())
                      then E.Ignore
                      else (
                        match result with
                        | Ok labels ->
                          set_candidate
                            (Some { Candidate.serial = request; expected; labels })
                        | Error error ->
                          E.bind
                            (Gpuio_eio.App.Window.Expert.palette_command
                               window
                               expected
                               ~if_query_unchanged:true
                               (Set_loading false))
                            ~f:(fun _ -> set_status (Error.to_string_hum error))))
                  |> Or_error.map ~f:Option.some)
            in
            (match started with
             | Ok None -> E.Ignore
             | Ok (Some running) -> E.of_thunk (fun () -> task := Some running)
             | Error error ->
               E.bind
                 (Gpuio_eio.App.Window.Expert.palette_command
                    window
                    expected
                    ~if_query_unchanged:true
                    (Set_loading false))
                 ~f:(fun _ -> set_status (Error.to_string_hum error)))))
    in
    E.Many [ Controller.observe controller expected; launch ]
  in
  let ids, labels =
    match candidate with
    | None -> [], []
    | Some candidate -> Candidate.commands candidate, candidate.Candidate.labels
  in
  let commands =
    List.map2_exn ids labels ~f:(fun id label ->
      Command.create
        ~id
        ~label
        ~on_invoke:(fun () -> set_selection ("Opened " ^ label))
        ()
      |> ok)
    |> Command.Registry.create
    |> ok
  in
  let chooser =
    if
      (not opened)
      ||
      match scope with
      | Preview_scope.Ready _ -> false
      | Loading | Failed _ -> true
    then V.column []
    else
      V.command_palette
        ~key:(Controller.key controller)
        ~config:
          (Command_palette.Config.create
             ~label:"Workspace search"
             ~placeholder:"Find notes and references…"
             ~search:External
             ~commands:ids
             ()
           |> ok)
        ~on_change:observe
        ~on_dismiss:(fun _ -> close)
        ()
      |> fun view ->
      V.with_palette_content
        view
        ~footer:(Palette.text p ~muted:true status)
        ~items:
          (List.map2_exn ids labels ~f:(fun command label ->
             ( command
             , V.column
                 [ Palette.text p label; Palette.text p ~muted:true "Workspace document" ]
             )))
        ()
      |> ok
  in
  V.command_scope
    ~commands
    [ Palette.card
        p
        ~title:"Search beyond the current view"
        [ Palette.text
            p
            ~muted:true
            "Find workspace notes and references while keeping the search field \
             responsive."
        ; Palette.button p "Search workspace" (set_opened true)
        ; Palette.text p selection
        ]
    ; chooser
    ]
;;
