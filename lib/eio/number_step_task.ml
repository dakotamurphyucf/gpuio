open Core
module N = Gpuio.Number_input
module E = Bonsai.Effect

module Error = struct
  type t =
    | Work_failed of Error.t
    | Resolution_failed of N.Command_error.t
  [@@deriving sexp_of]
end

module Phase = struct
  type t =
    | Working
    | Resolving
    | Finished
    | Cancelled
end

type t =
  { scope : Scope.t
  ; mutable phase : Phase.t
  ; mutable unregister : (unit -> unit) option
  }

let cancel t = Scope.cancel t.scope

let is_finished t =
  Scope.Expert.check t.scope;
  match t.phase with
  | Finished | Cancelled -> true
  | Working | Resolving -> false
;;

let finish t =
  t.phase <- Finished;
  let unregister = t.unregister in
  t.unregister <- None;
  Option.iter unregister ~f:(fun f -> f ());
  Scope.cancel t.scope
;;

let start ~scope ~resolve ~decline ~f ~on_result =
  match Scope.child scope ~name:"numeric-step" with
  | Error error ->
    decline ();
    Error error
  | Ok scope ->
    let t = { scope; phase = Working; unregister = None } in
    let cleanup () =
      t.unregister <- None;
      match t.phase with
      | Finished | Cancelled -> ()
      | Working | Resolving ->
        t.phase <- Cancelled;
        decline ()
    in
    (match Scope.on_cancel scope cleanup with
     | Error error ->
       finish t;
       decline ();
       Error error
     | Ok unregister ->
       t.unregister <- Some unregister;
       let complete result =
         let open E.Let_syntax in
         let%bind active =
           E.of_thunk (fun () ->
             match t.phase with
             | Working ->
               t.phase <- Resolving;
               true
             | Resolving | Finished | Cancelled -> false)
         in
         if not active
         then E.Ignore
         else (
           match result with
           | Error error ->
             let%bind () =
               E.of_thunk (fun () ->
                 decline ();
                 finish t)
             in
             on_result (Error (Error.Work_failed error))
           | Ok decision ->
             let%bind result = resolve decision in
             let%bind publish =
               E.of_thunk (fun () ->
                 match t.phase with
                 | Resolving ->
                   if Result.is_error result then decline ();
                   finish t;
                   true
                 | Working | Finished | Cancelled -> false)
             in
             if publish
             then
               on_result
                 (Result.map_error result ~f:(fun error -> Error.Resolution_failed error))
             else E.Ignore)
       in
       (match Scope.start scope ~f ~on_result:complete with
        | Ok _ -> Ok t
        | Error error ->
          cancel t;
          Error error))
;;
