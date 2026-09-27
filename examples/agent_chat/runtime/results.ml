open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module W = Gpuio_bonsai.Table
module Pager = Gpuio_eio.Table_paging
module D = Gpuio.Table_data
module C = Gpuio.Table_column
module T = Gpuio.Table
module Data = Result_data
module Q = Data.Query
module P = Gpuio.Presentation

type t =
  { pager : (Q.t, Data.Row.t) Pager.t
  ; fixture : Data.Row.t D.t Fixture_job.t
  ; build : Data.Row.t D.t -> Q.t -> Data.Row.t D.t
  ; query : Q.t B.Expert.Var.t
  ; columns : C.Collection.t B.Expert.Var.t
  ; busy : bool B.Expert.Var.t
  ; notice : string B.Expert.Var.t
  ; actions : Result_actions.t
  }

let ok = Or_error.ok_exn
let get = B.Expert.Var.get
let set = B.Expert.Var.set
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let create ~scope ~sleep ~build =
  let query = Q.create () |> ok in
  let previous = ref (0L, 0) in
  let%bind.Or_error pager =
    Pager.create
      ~scope
      ~query
      (D.create [] |> ok)
      ~before:End
      ~after:(More None)
      ~load:(fun request ->
        let query = Pager.Request.query request in
        let generation = Pager.Request.generation request in
        let old_generation, count = !previous in
        let attempt = if Int64.equal old_generation generation then count + 1 else 1 in
        previous := generation, attempt;
        sleep
          (match Q.loading query with
           | Normal | Fail_once -> 0.3
           | Slow -> 5.);
        match Q.loading query with
        | Fail_once when attempt = 1 ->
          Or_error.error_string "Sample results unavailable. Retry to continue."
        | Normal | Slow | Fail_once -> Data.page request)
  in
  let%map.Or_error fixture =
    match Fixture_job.create ~scope with
    | Ok fixture -> Ok fixture
    | Error error ->
      Pager.close pager;
      Error error
  in
  { pager
  ; fixture
  ; build
  ; query = B.Expert.Var.create query
  ; columns = B.Expert.Var.create (Data.columns ())
  ; busy = B.Expert.Var.create false
  ; notice = B.Expert.Var.create "Select a finding to inspect its result."
  ; actions = Result_actions.create ()
  }
;;

let accept t query source =
  set t.busy false;
  match Pager.reset t.pager ~query source ~before:End ~after:End with
  | Ok () -> set t.notice "Results updated."
  | Error error -> set t.notice (Error.to_string_hum error)
;;

let query t query =
  let open E.Let_syntax in
  let%bind source =
    E.of_thunk (fun () ->
      Pager.cancel t.pager Before;
      Pager.cancel t.pager After;
      Fixture_job.cancel t.fixture;
      set t.query query;
      set t.busy (Q.Size.equal (Q.size query) Large);
      set t.notice "Updating results…";
      (Pager.snapshot t.pager).data)
  in
  match Q.size query with
  | Sample -> E.of_thunk (fun () -> accept t query (Data.replace source query |> ok))
  | Large ->
    Fixture_job.submit
      t.fixture
      ~f:(fun () -> t.build source query)
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          match result with
          | Ok source -> accept t query source
          | Error error ->
            set t.busy false;
            set t.notice (Error.to_string_hum error)))
;;

let filter_scores t range =
  let open E.Let_syntax in
  let%bind current = E.of_thunk (fun () -> get t.query) in
  query t (Q.with_filter current (Between range))
;;

let paged_sample t loading =
  E.of_thunk (fun () ->
    Fixture_job.cancel t.fixture;
    set t.busy false;
    let query = Q.create ~loading () |> ok in
    set t.query query;
    match
      Pager.reset t.pager ~query (D.create [] |> ok) ~before:End ~after:(More None)
    with
    | Ok () -> set t.notice "Sample query ready for paging."
    | Error error -> set t.notice (Error.to_string_hum error))
;;

let columns t f =
  E.of_thunk (fun () ->
    match f (get t.columns) with
    | Ok columns -> set t.columns columns
    | Error error -> set t.notice (Error.to_string_hum error))
;;

let toggle_pin t =
  columns t (fun columns ->
    let number = C.Collection.find columns (Data.column "number") |> Option.value_exn in
    let pin = if C.Pin.equal (C.pin number) Left then C.Pin.Unpinned else Left in
    let%bind.Or_error number =
      C.create
        ~id:(C.id number)
        ~label:(C.label number)
        ~width:(C.width number)
        ~min_width:(C.min_width number)
        ~max_width:(C.max_width number)
        ~pin
        ~sortable:true
        ()
    in
    let remaining =
      C.Collection.to_list columns
      |> List.filter ~f:(fun column -> not (C.Id.equal (C.id column) (C.id number)))
    in
    Data.schema (number :: remaining))
;;

let request t request =
  let details =
    Result_actions.request
      t.actions
      ~generation:(Pager.snapshot t.pager).generation
      request
  in
  let action =
    match request with
    | T.Request.Resize widths ->
      columns t (fun columns ->
        List.fold_result widths ~init:columns ~f:(fun columns (column, width) ->
          C.Collection.resize columns ~column ~width))
    | Move (column, before) ->
      columns t (fun columns -> C.Collection.move columns ~column ~before)
    | Sort (column, direction) ->
      E.bind
        (E.of_thunk (fun () ->
           Q.with_sort
             (get t.query)
             (Option.map direction ~f:(fun direction -> { T.Sort.column; direction }))))
        ~f:(function
          | Ok next -> query t next
          | Error error -> E.of_thunk (fun () -> set t.notice (Error.to_string_hum error)))
    | Select _ | Activate _ | Context _ | Copy _ -> E.Ignore
  in
  E.Many [ details; action ]
;;

let component t ~active ~dark graph =
  let snapshot = Pager.value t.pager in
  let open B.Let_syntax in
  let config =
    let%arr columns = B.Expert.Var.value t.columns
    and snapshot = snapshot
    and busy = B.Expert.Var.value t.busy in
    T.Config.create
      ~columns
      ~label:"Run results"
      ?sort:(Q.sort snapshot.query)
      ~row_height:34.
      ~max_active_rows:24
      ~max_active_cells:96
      ~disabled:busy
      ~column_selection:true
      ()
    |> ok
  in
  let table_style =
    let%arr dark = dark in
    let palette = Palette.of_dark dark in
    style
      [ Width (Gpuio.Length.percent_exn 100.)
      ; Height (px 300.)
      ; Shrink 0.
      ; Background (Gpuio.Background.solid palette.surface)
      ; Foreground palette.text
      ; Border_width 1.
      ; Border_color palette.line
      ; Radius 10.
      ; Font_size 12.
      ]
    |> fun base ->
    Gpuio.Style.with_state_exn
      base
      Selected
      [ Background (Gpuio.Background.solid palette.accent_surface) ]
  in
  let auto_load =
    B.map2 active (B.Expert.Var.value t.busy) ~f:(fun active busy -> active && not busy)
  in
  let output =
    W.paged
      snapshot
      ~paging:(B.return (Pager.controls t.pager))
      ~config
      ~key:(Gpuio.Key.of_string_exn "run-results")
      ~style:table_style
      ~auto_load
      ~on_request:(B.return (request t))
      ~render_cell:(fun ~row:_ ~data ~column ~lifetime:_ _graph ->
        B.map2 data column ~f:(fun row column ->
          let%bind.Or_error text = Data.cell row (C.id column) in
          W.Cell.text ~column:(C.id column) text))
      graph
  in
  let actions =
    Result_actions.view
      t.actions
      ~snapshot
      ~current:(fun () -> Pager.snapshot t.pager)
      ~output
      ~describe:Data.describe
      ~result_column:(Data.column "summary")
      ~dark
      graph
  in
  let%arr output = output
  and snapshot = snapshot
  and actions = actions
  and dark = dark
  and requested = B.Expert.Var.value t.query
  and columns = B.Expert.Var.value t.columns
  and busy = B.Expert.Var.value t.busy
  and notice = B.Expert.Var.value t.notice in
  let palette = Palette.of_dark dark in
  let appearance =
    if dark
    then Gpuio.Presentation.Appearance.dark
    else Gpuio.Presentation.Appearance.light
  in
  let button ?(disabled = false) label on_click =
    V.button
      ~disabled
      ~on_click
      label
      ~style:
        (style
           [ Foreground palette.text
           ; Background (Gpuio.Background.solid palette.raised)
           ; Border_width 1.
           ; Border_color palette.line
           ; Padding (px 8.)
           ; Radius 8.
           ])
  in
  let selection =
    match output with
    | Error _ -> T.Selection.Empty
    | Ok output -> W.Output.selection output
  in
  let row_name row =
    D.find snapshot.data (D.Row_ref.id row)
    |> Option.value_map ~default:"Unavailable finding" ~f:(fun row ->
      sprintf "%06d" (Data.Row.number row))
  in
  let selection_label =
    match selection with
    | Empty -> "Select a finding to inspect it."
    | Row row -> "Selected result " ^ row_name row
    | Cell (row, column) ->
      sprintf "Selected result %s · %s" (row_name row) (C.Id.to_string column)
    | Column column -> "Selected column " ^ C.Id.to_string column
  in
  let reveal_last =
    match output, D.nth snapshot.data (D.length snapshot.data - 1) with
    | Ok output, Some (id, _) ->
      (match W.Output.target output id with
       | Error _ -> E.Ignore
       | Ok target ->
         W.Controller.batch
           (W.Output.controller output)
           [ Set_selection (Cell (target, Data.column "number"))
           ; Reveal (target, Some (Data.column "number"))
           ]
         |> ok)
    | Error _, _ | Ok _, None -> E.Ignore
  in
  let footer =
    match snapshot.after with
    | Failed error ->
      P.alert
        appearance
        ~tone:Warning
        ~title:"Results unavailable"
        ~actions:
          (button
             "Retry results"
             (W.Paging.retry
                (Pager.controls t.pager)
                ~generation:snapshot.generation
                After))
        [ V.text (Error.to_string_hum error) ]
    | Loading ->
      V.row
        ~style:(style [ Gap (px 8.); Align_items Center ])
        [ Query_loading.spinner ~dark ~label:"Loading result query"
        ; V.text "Loading results…"
        ]
    | Ready ->
      button
        "Load more results"
        (W.Paging.request (Pager.controls t.pager) ~generation:snapshot.generation After)
    | End ->
      V.text
        ~style:(style [ Foreground palette.muted; Font_size 12. ])
        "All results loaded"
  in
  let filter_label =
    match Q.filter snapshot.query with
    | All -> "Filter: all scores"
    | High_score -> "Filter: score ≥ 80%"
    | Empty -> "Filter: empty fixture"
    | Between range -> "Filter: score " ^ Score_range.describe range
  in
  let filtered =
    match Q.filter snapshot.query with
    | All -> false
    | High_score | Empty | Between _ -> true
  in
  let pending =
    match snapshot.after with
    | Loading -> true
    | Ready | End | Failed _ -> false
  in
  V.column
    ~style:(style [ Gap (px 14.); Min_width (px 0.) ])
    [ V.row
        [ Gpuio.Presentation.badge appearance ~size:Small ~tone:Accent "SIMULATED RESULTS"
        ]
    ; V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "Findings, in focus."
    ; P.banner
        appearance
        ~tone:Neutral
        ~live:Off
        ~title:"Simulated run · local data"
        ~style:
          (style
             [ Padding (px 10.); Radius 8.; Font_size 12.; Border_color palette.line ])
        [ V.text "Sort and inspect sample findings. No external service is contacted." ]
    ; (if filtered
       then
         P.tag
           appearance
           ~tone:Accent
           ~size:Small
           ~trailing:
             (V.button
                ~accessible_name:"Remove score filter"
                ~style:
                  (style
                     [ Border_width 0.
                     ; Padding (px 3.)
                     ; Foreground palette.accent
                     ; Background (Gpuio.Background.solid palette.accent_surface)
                     ])
                ~on_click:
                  (E.bind
                     (E.of_thunk (fun () -> Q.with_filter (get t.query) All))
                     ~f:(query t))
                "×")
           filter_label
       else
         P.label ~style:(style [ Foreground palette.muted; Font_size 12. ]) filter_label)
    ; (if pending && D.is_empty snapshot.data
       then Query_loading.results ~dark
       else if
         D.is_empty snapshot.data
         &&
         match snapshot.after with
         | End -> true
         | Ready | Loading | Failed _ -> false
       then
         Gpuio.Presentation.empty_state
           appearance
           ~title:"No findings match"
           ~description:"Try another score filter or restore the sample results."
           ~style:
             (style
                [ Height (px 220.)
                ; Background (Gpuio.Background.solid palette.surface)
                ; Border_color palette.line
                ; Radius 10.
                ])
           ~actions:(button "Restore sample results" (paged_sample t Normal))
           ()
       else (
         match output with
         | Error error -> V.text (Error.to_string_hum error)
         | Ok output -> W.Output.view output))
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 12. ])
        (sprintf
           "%s results loaded"
           (Int.to_string_hum ~delimiter:',' (D.length snapshot.data)))
    ; V.text
        ~style:(style [ Foreground palette.muted; Font_size 12. ])
        (match Q.sort snapshot.query with
         | None -> "Sort: run order"
         | Some sort ->
           sprintf
             "Sort: %s · %s"
             (C.Id.to_string sort.column)
             (match sort.direction with
              | Ascending -> "ascending"
              | Descending -> "descending"))
    ; P.status_bar
        appearance
        ~style:
          (style [ Padding (px 6.); Background (Gpuio.Background.solid palette.sidebar) ])
        ~leading:
          (P.marker
             appearance
             ~tone:(if busy || pending then Accent else Neutral)
             selection_label)
        ()
    ; footer
    ; (if busy
       then
         V.row
           ~style:(style [ Gap (px 8.); Align_items Center ])
           [ Query_loading.spinner ~dark ~label:"Building result fixture"
           ; V.text "Preparing results…"
           ]
       else V.text ~style:(style [ Foreground palette.accent; Font_size 12. ]) notice)
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button ~disabled:(D.is_empty snapshot.data) "Reveal last result" reveal_last
        ; button
            (if C.Collection.pinned_count columns = 0
             then "Pin result IDs"
             else "Unpin result IDs")
            (toggle_pin t)
        ; button "Reset columns" (E.of_thunk (fun () -> set t.columns (Data.columns ())))
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button "All scores" (query t (Q.with_filter requested All))
        ; button "Score ≥ 80" (query t (Q.with_filter requested High_score))
        ; button "Empty results" (query t (Q.with_filter requested Empty))
        ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ button "Paged sample" (paged_sample t Normal)
        ; button "Slow query" (paged_sample t Slow)
        ; button "Fail next query" (paged_sample t Fail_once)
        ; button "Load 100,000 results" (query t (Q.create ~size:Large () |> ok))
        ]
    ; actions
    ]
;;
