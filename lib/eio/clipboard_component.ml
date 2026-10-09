open Core
module Text = Gpuio.Clipboard.Text
module Error = Gpuio.Clipboard.Error
module B = Bonsai.Cont
module E = Bonsai.Effect

module Phase = struct
  type t =
    | Idle
    | Writing of Text.t
    | Copied of Text.t * Time_ns.t
    | Failed of Error.t
  [@@deriving equal]
end

module Model = struct
  type t =
    { serial : int64
    ; phase : Phase.t
    }
  [@@deriving equal]

  let empty = { serial = 0L; phase = Phase.Idle }

  let reset t =
    if Int64.equal t.serial Int64.max_value
    then { t with phase = Failed Native_failure }
    else { serial = Int64.succ t.serial; phase = Idle }
  ;;
end

module Action = struct
  type t =
    | Reset
    | Copy of int64
    | Finished of int64 * Text.t * (unit, Error.t) Result.t * Time_ns.t
    | Expired of int64
end

type input =
  { text : Text.t
  ; disabled : bool
  ; now : Time_ns.t E.t
  ; on_copied : Text.t -> unit E.t
  }

type t =
  { model : Model.t
  ; disabled : bool
  ; inject : Action.t -> unit E.t
  }

let create
      write
      ~text
      ?(disabled = B.return false)
      ?(on_copied = B.return (fun _ -> E.Ignore))
      graph
  =
  let open B.Let_syntax in
  let now = B.Clock.get_current_time graph in
  let input =
    let%arr text = text
    and disabled = disabled
    and now = now
    and on_copied = on_copied in
    { text; disabled; now; on_copied }
  in
  let model, inject =
    B.state_machine1
      ~default_model:Model.empty
      ~equal:Model.equal
      ~apply_action:(fun context input model action ->
        let schedule = B.Apply_action_context.schedule_event context in
        let inject = B.Apply_action_context.inject context in
        match action, input with
        | Action.Reset, _ -> Model.reset model
        | (Copy _ | Finished _ | Expired _), Bonsai.Computation_status.Inactive -> model
        | Copy serial, Active (input : input) ->
          if
            input.disabled
            || (not (Int64.equal serial model.serial))
            || Int64.equal model.serial Int64.max_value
          then model
          else (
            match model.phase with
            | Writing _ | Copied _ -> model
            | Idle | Failed _ ->
              let serial = Int64.succ model.serial in
              schedule
                (E.bind (write input.text) ~f:(fun result ->
                   E.bind input.now ~f:(fun now ->
                     inject (Finished (serial, input.text, result, now)))));
              { Model.serial; phase = Writing input.text })
        | Finished (serial, text, result, now), Active (input : input) ->
          if
            input.disabled
            || (not (Int64.equal serial model.serial))
            || not (Text.equal text input.text)
          then model
          else (
            match model.phase with
            | Writing expected when Text.equal expected text ->
              (match result with
               | Error error -> { model with phase = Failed error }
               | Ok () ->
                 schedule (input.on_copied text);
                 { model with
                   phase = Copied (text, Time_ns.add now (Time_ns.Span.of_sec 2.))
                 })
            | Idle | Writing _ | Copied _ | Failed _ -> model)
        | Expired serial, Active _ ->
          if Int64.equal serial model.serial
          then (
            match model.phase with
            | Copied _ -> Model.reset model
            | Idle | Writing _ | Failed _ -> model)
          else model)
      input
      graph
  in
  let changes = B.map input ~f:(fun i -> i.text, i.disabled) in
  B.Edge.on_change
    changes
    ~equal:[%equal: Text.t * bool]
    ~callback:(B.map inject ~f:(fun inject _ -> inject Reset))
    graph;
  B.Edge.lifecycle ~on_deactivate:(B.map inject ~f:(fun inject -> inject Reset)) graph;
  let deadline =
    B.map model ~f:(fun model ->
      match model.Model.phase with
      | Copied (_, until) -> until
      | Idle | Writing _ | Failed _ -> Time_ns.epoch)
  in
  let passed = B.Clock.at deadline graph in
  let expired =
    let%arr model = model
    and passed = passed in
    match model.Model.phase, passed with
    | Copied _, B.Clock.Before_or_after.After -> Some model.serial
    | (Idle | Writing _ | Failed _), _ | Copied _, Before -> None
  in
  B.Edge.on_change
    expired
    ~equal:[%equal: int64 option]
    ~callback:
      (B.map inject ~f:(fun inject -> function
         | None -> E.Ignore
         | Some serial -> inject (Expired serial)))
    graph;
  let%arr model = model
  and input = input
  and inject = inject in
  { model; disabled = input.disabled; inject }
;;

let is_busy t =
  match t.model.phase with
  | Writing _ -> true
  | Idle | Copied _ | Failed _ -> false
;;

let is_copied t =
  match t.model.phase with
  | Copied _ -> true
  | Idle | Writing _ | Failed _ -> false
;;

let error t =
  match t.model.phase with
  | Failed e -> Some e
  | Idle | Writing _ | Copied _ -> None
;;

let copy t = t.inject (Copy t.model.serial)

let view t ?(label = "Copy") ?(copied_label = "Copied") ?style () =
  Gpuio_bonsai.View.button
    ?style
    ~disabled:t.disabled
    ~config:(Gpuio.Button.Config.create ~loading:(is_busy t) ())
    ~on_click:(copy t)
    (if is_copied t then copied_label else label)
;;
