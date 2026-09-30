open Core
module App = Gpuio_eio.App
module D = Gpuio_eio.Desktop
module N = Gpuio_eio.Notification
module Content = Gpuio.Notification
module B = Bonsai.Cont
module E = Bonsai.Effect

let ok = Or_error.ok_exn
let scheme = Gpuio.Deep_link.Scheme.of_string "gpuio-studio" |> ok

let identity =
  D.Identity.create
    ~identifier:"com.gpuio.component-studio"
    ~name:"GPUIO Component Studio"
    ~schemes:[ scheme ]
    ()
  |> ok
;;

let tag = N.Tag.of_string "studio-preview" |> ok

let content ~updated =
  Content.create
    ~title:(if updated then "Your preview has an update" else "Your preview is ready")
    ~body:"A small notification from Component Studio. 日本語 👋"
    ~actions:
      [ Content.Action.create
          (Content.Action_id.of_string "inspect" |> ok)
          ~label:"Inspect preview"
        |> ok
      ]
    ()
  |> ok
;;

module Snapshot = struct
  type t =
    { desktop_support : string
    ; notification_support : string
    ; authorization : string
    ; link : string
    ; notice : string
    ; busy : bool
    ; has_receipt : bool
    }
end

type t =
  { app : App.t
  ; desktop : D.t
  ; notifications : N.t
  ; state : Snapshot.t B.Expert.Var.t
  ; receipt : N.Receipt.t option ref
  }

let update state f = B.Expert.Var.set state (f (B.Expert.Var.get state))
let error sexp value = Sexp.to_string_hum (sexp value)

let report label sexp = function
  | Ok () -> label ^ ": request accepted"
  | Error e -> label ^ ": " ^ error sexp e
;;

let supported pairs =
  List.filter_map pairs ~f:(fun (name, supported) ->
    if supported then Some name else None)
  |> String.concat ~sep:", "
;;

let create app =
  let state =
    B.Expert.Var.create
      { Snapshot.desktop_support = "Desktop support: not checked"
      ; notification_support = "Notification support: not checked"
      ; authorization = "Notification access: not checked"
      ; link = "No incoming link yet"
      ; notice = "No notification requested"
      ; busy = false
      ; has_receipt = false
      }
  in
  let receipt = ref None in
  let desktop =
    D.attach app ~on_event:(fun event ->
      E.of_thunk (fun () ->
        let link =
          match event with
          | Link link -> "Received link: " ^ Gpuio.Deep_link.to_string link
          | Rejected_link { input = _; reason } ->
            "Rejected link: " ^ error Gpuio.Deep_link.Error.sexp_of_t reason
          | Overflow count -> sprintf "Incoming links dropped: %Ld" count
          | Failed e -> "Link intake: " ^ error D.Error.sexp_of_t e
        in
        update state (fun s -> { s with link })))
    |> function
    | Ok t -> t
    | Error e -> raise_s [%sexp (e : D.Error.t)]
  in
  let notifications =
    N.attach app ~on_event:(fun event ->
      E.of_thunk (fun () ->
        let delivered, notice =
          match event with
          | Activated r -> Some r, "Notification opened"
          | Action (r, action) ->
            Some r, "Notification action: " ^ Content.Action_id.to_string action
          | Closed (r, reason) ->
            Some r, "Notification closed: " ^ error Content.Closed_reason.sexp_of_t reason
          | Failed e -> None, "Notification intake: " ^ error N.Error.sexp_of_t e
        in
        let current =
          Option.is_some delivered && Option.equal N.Receipt.equal !receipt delivered
        in
        if current then receipt := None;
        if current || Option.is_none delivered
        then
          update state (fun s -> { s with notice; has_receipt = Option.is_some !receipt })))
    |> function
    | Ok t -> t
    | Error e -> raise_s [%sexp (e : N.Error.t)]
  in
  { app; desktop; notifications; state; receipt }
;;

let ready t =
  D.ready t.desktop;
  N.ready t.notifications
;;

let snapshot t = B.Expert.Var.value t.state

(* Read admission at execution, not render time: multiple windows share a lane. *)
let serial t f =
  E.bind
    (E.of_thunk (fun () ->
       let available = not (B.Expert.Var.get t.state).busy in
       if available then update t.state (fun s -> { s with busy = true });
       available))
    ~f:(fun available ->
      if not available
      then E.Ignore
      else E.map (f ()) ~f:(fun () -> update t.state (fun s -> { s with busy = false })))
;;

let check t =
  serial t (fun () ->
    E.bind (D.capabilities t.app) ~f:(fun result ->
      update t.state (fun s ->
        { s with
          desktop_support =
            ("Desktop support: "
             ^
             match result with
             | Error e -> error D.Error.sexp_of_t e
             | Ok c ->
               supported
                 [ "links", c.incoming_links
                 ; "registration", c.runtime_registration
                 ; "activation", c.application_activation
                 ; "reveal", c.file_reveal
                 ; "open", c.file_open
                 ; "document metadata", c.document_metadata
                 ])
        });
      E.bind (N.capabilities t.notifications) ~f:(fun result ->
        update t.state (fun s ->
          { s with
            notification_support =
              ("Notification support: "
               ^
               match result with
               | Error e -> error N.Error.sexp_of_t e
               | Ok c ->
                 supported
                   [ "body", c.body
                   ; "actions", c.actions
                   ; "activation", c.activation
                   ; "replacement", c.replacement
                   ; "dismissal", c.dismissal
                   ; "permission request", c.permission_request
                   ; "sound", c.sound
                   ])
          });
        E.map (N.authorization t.notifications) ~f:(fun result ->
          update t.state (fun s ->
            { s with
              authorization =
                ("Notification access: "
                 ^
                 match result with
                 | Ok a -> error N.Authorization.sexp_of_t a
                 | Error e -> error N.Error.sexp_of_t e)
            })))))
;;

let allow t =
  serial t (fun () ->
    E.map (N.request_authorization t.notifications) ~f:(fun result ->
      update t.state (fun s ->
        { s with
          authorization =
            ("Notification access: "
             ^
             match result with
             | Ok a -> error N.Authorization.sexp_of_t a
             | Error e -> error N.Error.sexp_of_t e)
        })))
;;

let post t =
  serial t (fun () ->
    match !(t.receipt) with
    | Some _ -> E.Ignore
    | None ->
      E.map
        (N.post t.notifications ~tag (content ~updated:false))
        ~f:(fun result ->
          let notice =
            match result with
            | Ok r ->
              t.receipt := Some r;
              "Notification submitted; presentation is decided by the OS"
            | Error e -> "Notification post: " ^ error N.Error.sexp_of_t e
          in
          update t.state (fun s ->
            { s with notice; has_receipt = Option.is_some !(t.receipt) })))
;;

let replace t =
  serial t (fun () ->
    match !(t.receipt) with
    | None -> E.Ignore
    | Some receipt ->
      E.map
        (N.replace t.notifications receipt (content ~updated:true))
        ~f:(fun result ->
          update t.state (fun s ->
            { s with notice = report "Notification replacement" N.Error.sexp_of_t result })))
;;

let dismiss t =
  serial t (fun () ->
    match !(t.receipt) with
    | None -> E.Ignore
    | Some receipt ->
      E.map (N.dismiss t.notifications receipt) ~f:(fun result ->
        (match result with
         | Ok () when Option.equal N.Receipt.equal !(t.receipt) (Some receipt) ->
           t.receipt := None
         | Ok () | Error _ -> ());
        update t.state (fun s ->
          { s with
            notice = report "Notification dismissal" N.Error.sexp_of_t result
          ; has_receipt = Option.is_some !(t.receipt)
          })))
;;

let retry t =
  E.of_thunk (fun () ->
    D.retry t.desktop;
    N.retry t.notifications)
;;
