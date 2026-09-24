open Core
module B = Bonsai.Cont
module E = Bonsai.Effect

module Lifetime = struct
  type t = { mutable active : bool }

  let guard t action =
    E.bind
      (E.of_thunk (fun () -> t.active))
      ~f:(fun active -> if active then action else E.Ignore)
  ;;
end

let assoc comparator input ~f graph =
  B.assoc
    comparator
    input
    ~f:(fun key data graph ->
      let pair, reset =
        B.with_model_resetter
          ~f:(fun graph ->
            (* The row's own activation hooks run before the wrapper's later
               lifecycle path. Start valid so those hooks may use the guard. *)
            let lifetime =
              B.Expert.thunk ~f:(fun () -> { Lifetime.active = true }) graph
            in
            let result = f key data lifetime graph in
            B.both result lifetime)
          graph
      in
      let result = B.map pair ~f:fst in
      let lifetime = B.map pair ~f:snd in
      let open B.Let_syntax in
      let on_deactivate =
        let%arr lifetime = lifetime
        and reset = reset in
        E.Many [ E.of_thunk (fun () -> lifetime.active <- false); reset ]
      in
      B.Edge.lifecycle ~on_deactivate graph;
      result)
    graph
;;
