open Core
module Scope = Gpuio_eio.Scope
module E = Bonsai.Effect

module Request = struct
  type 'a t =
    { serial : int
    ; run : unit -> 'a
    ; complete : 'a Or_error.t -> unit E.t
    }
end

type 'a t =
  { scope : Scope.t
  ; mutable serial : int
  ; mutable running : bool
  ; mutable pending : 'a Request.t option
  }

let cancel t =
  t.serial <- t.serial + 1;
  t.pending <- None
;;

let create ~scope =
  let t = { scope; serial = 0; running = false; pending = None } in
  let%map.Or_error _unregister = Scope.on_cancel scope (fun () -> cancel t) in
  t
;;

let rec pump t =
  let open E.Let_syntax in
  let%bind request =
    E.of_thunk (fun () ->
      if t.running || not (Scope.is_active t.scope)
      then None
      else (
        let pending = t.pending in
        t.pending <- None;
        if Option.is_some pending then t.running <- true;
        pending))
  in
  match request with
  | None -> E.Ignore
  | Some request ->
    let complete result =
      let%bind current =
        E.of_thunk (fun () ->
          t.running <- false;
          request.serial = t.serial && Scope.is_active t.scope)
      in
      let%bind () = if current then request.complete result else E.Ignore in
      pump t
    in
    let%bind started =
      E.of_thunk (fun () -> Scope.start t.scope ~f:request.run ~on_result:complete)
    in
    (match started with
     | Ok _ -> E.Ignore
     | Error error -> complete (Error error))
;;

let submit t ~f ~on_result =
  let open E.Let_syntax in
  let%bind () =
    E.of_thunk (fun () ->
      if Scope.is_active t.scope
      then (
        t.serial <- t.serial + 1;
        t.pending <- Some { Request.serial = t.serial; run = f; complete = on_result }))
  in
  pump t
;;
