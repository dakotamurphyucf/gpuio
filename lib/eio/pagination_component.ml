open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module P = Gpuio.Pagination
module N = Gpuio.Number_input
module L = Gpuio.Navigation.Pagination_layout
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let key = Gpuio.Key.of_string_exn
let style = Gpuio.Style.create_exn
let px = Gpuio.Length.px_exn

type command =
  N.Snapshot.t -> N.Command.t -> (N.Snapshot.t, N.Command_error.t) Result.t E.t

type opening =
  { id : int
  ; source : P.t
  ; first : int
  ; last : int
  ; snapshot : N.Snapshot.t option
  ; pending : bool
  ; error : N.Command_error.t option
  }
[@@deriving equal]

type model =
  { next_id : int
  ; opening : opening option
  }
[@@deriving equal]

type input =
  { pages : P.t
  ; layout : L.t
  ; on_request : P.Request.t -> unit E.t
  }

type action =
  | Sync
  | Open of P.t * int * int
  | Cancel of int
  | Observe of int * N.Snapshot.t
  | Choose of int * int
  | Confirm of int
  | Finished of int * N.Snapshot.t * (N.Snapshot.t, N.Command_error.t) Result.t

type t =
  { key : string
  ; input : input
  ; opening : opening option
  ; inject : action -> unit E.t
  }

let sync (model : model) input =
  match model.opening with
  | Some opening
    when (not (P.equal opening.source input.pages)) || L.equal input.layout Compact ->
    { model with opening = None }
  | _ -> model
;;

let same_owner a b =
  Gpuio_protocol.Window_id.equal (N.Expert.window a) (N.Expert.window b)
  && Gpuio_protocol.Node_id.equal (N.Expert.node a) (N.Expert.node b)
;;

let domain opening =
  Gpuio.Numeric.Domain.create
    ~min:(Float.of_int opening.first)
    ~max:(Float.of_int opening.last)
    ~step:1.
  |> ok
;;

let accepts opening snapshot =
  Gpuio.Numeric.Domain.equal (domain opening) (N.Snapshot.domain snapshot)
  && Option.value_map opening.snapshot ~default:true ~f:(fun previous ->
    same_owner previous snapshot
    && N.Revision.compare (N.Snapshot.revision snapshot) (N.Snapshot.revision previous)
       >= 0)
;;

let create command ~model:pages ?(layout = B.return L.Full) ~on_request graph =
  let open B.Let_syntax in
  let input =
    let%arr pages = pages
    and layout = layout
    and on_request = on_request in
    { pages; layout; on_request }
  in
  let model, inject =
    B.state_machine1
      ~default_model:{ next_id = 0; opening = None }
      ~equal:equal_model
      ~apply_action:(fun context input model action ->
        let schedule = B.Apply_action_context.schedule_event context in
        let inject = B.Apply_action_context.inject context in
        match input with
        | Bonsai.Computation_status.Inactive -> { model with opening = None }
        | Active input ->
          let model = sync model input in
          let update opening = { model with opening = Some opening } in
          let choose opening page =
            if page < opening.first || page > opening.last
            then model
            else (
              schedule (input.on_request (P.Request.page page |> ok));
              { model with opening = None })
          in
          (match action with
           | Sync -> model
           | Open (source, first, last) ->
             if
               (not (P.equal source input.pages))
               || P.is_disabled source
               || L.equal input.layout Compact
               || model.next_id = Int.max_value
               || not
                    (List.exists (P.items source) ~f:(function
                       | Gap gap -> gap.first = first && gap.last = last
                       | Page _ -> false))
             then model
             else
               { next_id = model.next_id + 1
               ; opening =
                   Some
                     { id = model.next_id
                     ; source
                     ; first
                     ; last
                     ; snapshot = None
                     ; pending = false
                     ; error = None
                     }
               }
           | Cancel id ->
             if Option.exists model.opening ~f:(fun o -> o.id = id)
             then { model with opening = None }
             else model
           | Observe (id, snapshot) ->
             (match model.opening with
              | Some o when o.id = id && accepts o snapshot ->
                update { o with snapshot = Some snapshot; error = None }
              | _ -> model)
           | Choose (id, page) ->
             (match model.opening with
              | Some o when o.id = id -> choose o page
              | _ -> model)
           | Confirm id ->
             (match model.opening with
              | Some o when o.id = id && not o.pending ->
                (match o.snapshot with
                 | None -> update { o with error = Some Not_mounted }
                 | Some expected ->
                   schedule
                     (let open E.Let_syntax in
                      let%bind reply = command expected N.Command.Commit in
                      inject (Finished (id, expected, reply)));
                   update { o with pending = true; error = None })
              | _ -> model)
           | Finished (id, expected, reply) ->
             (match model.opening with
              | Some o when o.id = id && o.pending ->
                let fail error = update { o with pending = false; error = Some error } in
                (match reply with
                 | Error error -> fail error
                 | Ok snapshot ->
                   if not (same_owner expected snapshot && accepts o snapshot)
                   then fail Stale_revision
                   else if Option.is_some (N.Snapshot.composition snapshot)
                   then fail Composing
                   else (
                     match
                       N.Snapshot.committed snapshot, N.Snapshot.classification snapshot
                     with
                     | Number page, Valid draft
                       when Float.equal page draft
                            && Float.equal page (Float.round_down page) ->
                       choose o (Float.to_int page)
                     | _ -> fail Invalid_value))
              | _ -> model)))
      input
      graph
  in
  let changes = B.map input ~f:(fun i -> i.pages, i.layout) in
  B.Edge.on_change
    changes
    ~equal:[%equal: P.t * L.t]
    ~callback:(B.map inject ~f:(fun inject _ -> inject Sync))
    graph;
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr model = model
       and inject = inject in
       Option.value_map model.opening ~default:E.Ignore ~f:(fun o -> inject (Cancel o.id)))
    graph;
  let path = B.path_id graph in
  let%arr model = model
  and input = input
  and inject = inject
  and key = path in
  { key; input; opening = (sync model input).opening; inject }
;;

let is_open t = Option.is_some t.opening
let is_confirming t = Option.exists t.opening ~f:(fun o -> o.pending)
let error t = Option.bind t.opening ~f:(fun o -> o.error)

let cancel t =
  Option.value_map t.opening ~default:E.Ignore ~f:(fun o -> t.inject (Cancel o.id))
;;

let confirm t =
  Option.value_map t.opening ~default:E.Ignore ~f:(fun o -> t.inject (Confirm o.id))
;;

let shortcuts o =
  List.dedup_and_sort
    ~compare:Int.compare
    [ o.first
    ; Int.min o.last (o.first + 1)
    ; Int.min o.last (o.first + 2)
    ; o.first + ((o.last - o.first) / 2)
    ; Int.max o.first (o.last - 2)
    ; Int.max o.first (o.last - 1)
    ; o.last
    ]
;;

let view ?style:custom ?panel_style ?appearance ?labels ~overlay t =
  let content =
    Option.map t.opening ~f:(fun o ->
      let config =
        N.Config.create
          ~domain:(domain o)
          ~label:"Page number"
          ~step_controls:Hidden
          ~auto_focus:true
          ()
        |> ok
      in
      let on_event event =
        let snapshot =
          match event with
          | N.Event.Observed s
          | Changed s
          | Committed (_, s)
          | Rejected (_, s)
          | Cancelled (_, s) -> s
          | Step_requested request -> N.Step_request.snapshot request
        in
        t.inject (Observe (o.id, snapshot))
      in
      V.column
        ~style:
          (Gpuio.Style.merge
             [ style [ Gap (px 12.); Padding (px 12.) ]
             ; Option.value panel_style ~default:Gpuio.Style.empty
             ])
        [ V.text (sprintf "Choose a page from %d to %d" o.first o.last)
        ; V.row
            ~style:(style [ Gap (px 6.); Wrap Wrap ])
            (List.map (shortcuts o) ~f:(fun page ->
               V.button
                 ~key:(key (Int.to_string page))
                 ~on_click:(t.inject (Choose (o.id, page)))
                 (Int.to_string page)))
        ; Gpuio.View.number_input
            ~controller:(key (t.key ^ ":page:" ^ Int.to_string o.id))
            ~config
            ~initial:(N.Value.of_float (Float.of_int o.first) |> ok)
            ~on_event
            ()
        ; V.row
            ~style:(style [ Gap (px 8.) ])
            [ V.button ~on_click:(cancel t) "Cancel"
            ; V.button
                ~disabled:(o.pending || Option.is_none o.snapshot)
                ~on_click:(confirm t)
                "Go to page"
            ]
        ; (match o.error with
           | None -> V.column []
           | Some _ ->
             V.with_accessibility
               (V.text "Unable to select this page. Check the value and try again.")
               (Gpuio.Accessibility.create ~role:Alert () |> ok)
             |> ok)
        ])
  in
  Gpuio.Navigation.pagination
    t.input.pages
    ~key:(key (t.key ^ ":pagination"))
    ?style:custom
    ?appearance
    ?labels
    ~layout:t.input.layout
    ~on_request:t.input.on_request
    ~on_gap:(fun ~first ~last -> t.inject (Open (t.input.pages, first, last)))
    ~gap_popup:(fun ~first ~last ->
      let content =
        match t.opening with
        | Some o when o.first = first && o.last = last -> content
        | None | Some _ -> None
      in
      Gpuio.Navigation.Gap_popup.create
        ~config:overlay
        ~on_dismiss:(fun _ -> cancel t)
        content)
    ()
;;
