open Core
module Guard = Gpuio_runtime_core.Domain_guard

exception Scope_closed

module Task = struct
  type t =
    { guard : Guard.t
    ; mutable cancelled : bool
    ; mutable finished : bool
    ; mutable context : Eio.Cancel.t option
    }

  let cancel t =
    Guard.check t.guard;
    t.cancelled <- true;
    Option.iter t.context ~f:(fun context -> Eio.Cancel.cancel context Scope_closed)
  ;;

  let is_finished t =
    Guard.check t.guard;
    t.finished
  ;;
end

type shared =
  { guard : Guard.t
  ; sw : Eio.Switch.t
  ; inbox : Inbox.t
  ; max_tasks : int
  ; mutable tasks : int
  ; mutable scopes : int
  ; mutable cleanups : int
  ; mutable next : int
  }

type t =
  { shared : shared
  ; id : int
  ; parent : t option
  ; mutable active : bool
  ; mutable children : t Int.Map.t
  ; mutable tasks : Task.t Int.Map.t
  ; mutable cleanups : (unit -> unit) Int.Map.t
  }

let check t = Guard.check t.shared.guard

let is_active t =
  check t;
  t.active
;;

let fresh shared =
  let id = shared.next in
  if id = Int.max_value then failwith "task identity exhausted";
  shared.next <- id + 1;
  id
;;

let child t ~name =
  check t;
  if not t.active
  then Or_error.error_string "parent task scope closed"
  else if String.is_empty name || String.length name > 256
  then Or_error.error_string "scope name must contain 1..256 bytes"
  else if t.shared.scopes >= 1024
  then Or_error.error_string "task scope limit reached"
  else (
    let id = fresh t.shared in
    let child =
      { shared = t.shared
      ; id
      ; parent = Some t
      ; active = true
      ; children = Int.Map.empty
      ; tasks = Int.Map.empty
      ; cleanups = Int.Map.empty
      }
    in
    t.shared.scopes <- t.shared.scopes + 1;
    t.children <- Map.set t.children ~key:id ~data:child;
    Ok child)
;;

let rec cancel t =
  check t;
  if t.active
  then (
    t.active <- false;
    Map.iter t.children ~f:cancel;
    t.children <- Int.Map.empty;
    Map.iter t.tasks ~f:Task.cancel;
    let cleanups = t.cleanups in
    t.cleanups <- Int.Map.empty;
    t.shared.cleanups <- t.shared.cleanups - Map.length cleanups;
    Map.iter cleanups ~f:(fun cleanup -> cleanup ());
    t.shared.scopes <- t.shared.scopes - 1;
    Option.iter t.parent ~f:(fun parent ->
      parent.children <- Map.remove parent.children t.id))
;;

let enqueue t job =
  check t;
  if t.active then Inbox.push t.shared.inbox (fun () -> if t.active then job ())
;;

let on_cancel t cleanup =
  check t;
  if not t.active
  then Or_error.error_string "task scope closed"
  else if t.shared.cleanups >= 4096
  then Or_error.error_string "scoped resource limit reached"
  else (
    let id = fresh t.shared in
    t.cleanups <- Map.set t.cleanups ~key:id ~data:cleanup;
    t.shared.cleanups <- t.shared.cleanups + 1;
    Ok
      (fun () ->
        check t;
        if Map.mem t.cleanups id
        then (
          t.cleanups <- Map.remove t.cleanups id;
          t.shared.cleanups <- t.shared.cleanups - 1)))
;;

let start t ~f ~on_result =
  check t;
  if not t.active
  then Or_error.error_string "task scope closed"
  else if t.shared.tasks >= t.shared.max_tasks
  then Or_error.error_string "task limit reached"
  else (
    let id = fresh t.shared in
    let task =
      { Task.guard = t.shared.guard; cancelled = false; finished = false; context = None }
    in
    t.shared.tasks <- t.shared.tasks + 1;
    t.tasks <- Map.set t.tasks ~key:id ~data:task;
    Eio.Fiber.fork ~sw:t.shared.sw (fun () ->
      Exn.protect
        ~finally:(fun () ->
          task.context <- None;
          task.finished <- true;
          t.tasks <- Map.remove t.tasks id;
          t.shared.tasks <- t.shared.tasks - 1)
        ~f:(fun () ->
          try
            Eio.Cancel.sub (fun context ->
              task.context <- Some context;
              if (not task.cancelled) && t.active
              then (
                let result =
                  try Ok (f ()) with
                  | Eio.Cancel.Cancelled _ as exn -> raise exn
                  | exn -> Error (Error.of_exn exn)
                in
                enqueue t (fun () ->
                  if not task.cancelled
                  then Bonsai.Effect.Expert.handle (on_result result))))
          with
          | Eio.Cancel.Cancelled _ as exn -> if not task.cancelled then raise exn));
    Ok task)
;;

module Expert = struct
  let create ~sw ~inbox ~max_tasks =
    if max_tasks < 1 then invalid_arg "max_tasks";
    let shared =
      { guard = Guard.create ()
      ; sw
      ; inbox
      ; max_tasks
      ; tasks = 0
      ; scopes = 1
      ; cleanups = 0
      ; next = 1
      }
    in
    { shared
    ; id = 0
    ; parent = None
    ; active = true
    ; children = Int.Map.empty
    ; tasks = Int.Map.empty
    ; cleanups = Int.Map.empty
    }
  ;;

  let same_tree a b =
    check a;
    check b;
    phys_equal a.shared b.shared
  ;;

  let on_cancel = on_cancel

  let try_enqueue t job =
    check t;
    t.active && Inbox.try_push t.shared.inbox (fun () -> if t.active then job ())
  ;;

  let enqueue = enqueue
  let check = check
end
