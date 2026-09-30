open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Scope = Gpuio_eio.Scope

type 'a t =
  | Loading
  | Ready of 'a
  | Failed of Error.t

let acquire window ~name ~create graph =
  let state, set_state = B.state Loading graph in
  (* One owner per constructed branch, never shared between windows. *)
  let current = ref None in
  let cancel () =
    Option.iter !current ~f:Scope.cancel;
    current := None
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_activate:
      (let%arr set_state = set_state in
       E.bind
         (E.of_thunk (fun () ->
            cancel ();
            let result = Scope.child (Gpuio_eio.App.Window.scope window) ~name in
            current := Or_error.ok result;
            result))
         ~f:(function
           | Error error -> set_state (Failed error)
           | Ok scope ->
             E.bind (create scope) ~f:(fun result ->
               if not (Scope.is_active scope)
               then E.Ignore
               else (
                 match result with
                 | Ok value -> set_state (Ready value)
                 | Error error -> E.Many [ E.of_thunk cancel; set_state (Failed error) ]))))
    ~on_deactivate:
      (let%arr set_state = set_state in
       E.Many [ E.of_thunk cancel; set_state Loading ])
    graph;
  state
;;
