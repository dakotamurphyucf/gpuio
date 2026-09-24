open Core
module B = Bonsai.Cont
module R = Gpuio.Reconciler
module Wire = Gpuio_protocol.Wire

type pending =
  { update : unit Bonsai.Effect.t R.update
  ; revision : int64
  ; lifecycles : Bonsai_driver.Lifecycle_snapshot.t
  ; mutable submitted : bool
  }

type t =
  { guard : Domain_guard.t
  ; driver : unit Bonsai.Effect.t Gpuio.View.t option Bonsai_driver.t
  ; active : bool B.Expert.Var.t
  ; clock : Bonsai.Time_source.t
  ; reconciler : unit Bonsai.Effect.t R.t
  ; mutable theme : Gpuio.Theme.t
  ; mutable pending : pending option
  ; mutable closed : bool
  ; mutable cycles : int
  }

let create ?asset_owner window ~start ~theme component =
  let active = B.Expert.Var.create true in
  let computation graph =
    let open B.Let_syntax in
    match%sub B.Expert.Var.value active with
    | true ->
      let%arr view = component graph in
      Some view
    | false -> B.return None
  in
  let clock = Bonsai.Time_source.create ~start in
  { guard = Domain_guard.create ()
  ; driver = Bonsai_driver.create ~clock computation
  ; active
  ; clock
  ; reconciler = R.create ?asset_owner window
  ; theme
  ; pending = None
  ; closed = false
  ; cycles = 0
  }
;;

let check t = Domain_guard.check t.guard

let capture t =
  let open Or_error.Let_syntax in
  let%map update =
    R.prepare t.reconciler ~theme:t.theme (Bonsai_driver.result t.driver)
  in
  match R.message update with
  | None ->
    R.accept t.reconciler update |> Or_error.ok_exn;
    false
  | Some (Wire.Message.Apply tx) ->
    t.pending
    <- Some
         { update
         ; revision = tx.revision
         ; lifecycles = Bonsai_driver.Expert.snapshot_lifecycles t.driver
         ; submitted = false
         };
    true
  | Some _ -> assert false
;;

let advance_clock t ~now =
  check t;
  if not t.closed
  then
    Bonsai.Time_source.advance_clock
      t.clock
      ~to_:(Time_ns.max now (Bonsai.Time_source.now t.clock))
;;

let cycle t ~now =
  check t;
  if t.closed
  then Or_error.error_string "window driver closed"
  else (
    t.cycles <- t.cycles + 1;
    advance_clock t ~now;
    match t.pending with
    | Some _ -> Ok ()
    | None ->
      (* The lifecycle snapshot must describe the submitted native candidate.
         Buffer model actions while a commit is pending; accepting it must not
         activate/deactivate a newer graph that native code has never applied. *)
      Bonsai_driver.flush t.driver;
      let open Or_error.Let_syntax in
      let%bind has_message = capture t in
      if has_message
      then Ok ()
      else (
        Bonsai_driver.trigger_lifecycles t.driver;
        (* Lifecycle effects may inject actions even when there was no native diff.
       Flush once more now; further lifecycle turns belong to the next wakeup. *)
        if t.closed
        then Ok ()
        else (
          Bonsai_driver.flush t.driver;
          let%map (_ : bool) = capture t in
          ())))
;;

let next_message t =
  check t;
  Option.bind t.pending ~f:(fun pending ->
    if pending.submitted then None else R.message pending.update)
;;

let submitted t =
  check t;
  match t.pending with
  | Some pending -> pending.submitted <- true
  | None -> invalid_arg "no prepared transaction"
;;

let acknowledge t ~revision =
  check t;
  match t.pending with
  | Some pending when pending.submitted && Int64.equal revision pending.revision ->
    let open Or_error.Let_syntax in
    let%map () = R.accept t.reconciler pending.update in
    t.pending <- None;
    Bonsai_driver.Lifecycle_snapshot.trigger pending.lifecycles
  | Some _ | None -> Or_error.error_string "unexpected native acceptance"
;;

let schedule t action =
  check t;
  if not t.closed then Bonsai_driver.schedule_event t.driver action
;;

let retry_list_rows t ~revision notices =
  check t;
  match t.pending with
  | Some pending when pending.submitted && Int64.equal revision pending.revision ->
    let open Or_error.Let_syntax in
    let%map actions = R.retain_list_rows t.reconciler notices in
    t.pending <- None;
    List.iter actions ~f:(schedule t)
  | Some _ | None -> Or_error.error_string "unexpected native list retention response"
;;

let dispatch t event =
  check t;
  if not t.closed then Option.iter (R.dispatch t.reconciler event) ~f:(schedule t)
;;

let set_theme t theme =
  check t;
  t.theme <- theme
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    t.pending <- None;
    Exn.protect
      ~f:(fun () ->
        B.Expert.Var.set t.active false;
        Bonsai_driver.flush t.driver;
        Bonsai_driver.trigger_lifecycles t.driver;
        Bonsai_driver.flush t.driver)
      ~finally:(fun () ->
        R.close t.reconciler;
        Bonsai_driver.Expert.invalidate_observers t.driver))
;;

let revision t =
  check t;
  R.revision t.reconciler
;;

let cycles t =
  check t;
  t.cycles
;;
