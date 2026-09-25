open Core
module Backend = Gpuio_agent_chat_model.Fake_backend
module Pager = Gpuio_eio.List_paging
module Scope = Gpuio_eio.Scope
module Document = Gpuio_eio.Document
module App = Gpuio_eio.App
module Source = Gpuio.Text_source
module Collection = Gpuio.List_collection
module B = Bonsai.Cont
module E = Bonsai.Effect

module Phase = struct
  type t =
    | Idle
    | Accepting
    | Streaming
    | Complete
    | Cancelled
    | Failed of string
  [@@deriving equal, sexp_of]
end

module Message = struct
  type body =
    | Plain of string
    | Rich of Document.t * Gpuio.Document.Mode.t

  type t =
    { author : string
    ; body : body
    ; detail : string
    }
end

module Response = struct
  type t =
    { document : Document.t
    ; row : int
    ; prompt : string
    }
end

module Active = struct
  type t =
    { token : int
    ; scope : Scope.t
    ; mutable response : Response.t option
    }
end

type t =
  { app : App.t
  ; scope : Scope.t
  ; id : int
  ; title : string
  ; sleep : float -> unit
  ; pager : (int, Message.t, Int.comparator_witness) Pager.t
  ; phase : Phase.t B.Expert.Var.t
  ; mutable active : Active.t option
  ; mutable token : int
  ; mutable next_row : int
  ; mutable responses : int
  ; mutable last : Response.t option
  ; mutable attachments : int
  ; mutable initialized : bool
  }

let id t = t.id
let title t = t.title
let scope t = t.scope
let pager t = t.pager
let phase t = B.Expert.Var.get t.phase
let phase_value t = B.Expert.Var.value t.phase
let set_phase t phase = B.Expert.Var.set t.phase phase
let response_count t = t.responses
let last_document t = Option.map t.last ~f:(fun response -> response.document)

let is_busy t =
  match phase t with
  | Accepting | Streaming -> true
  | Idle | Complete | Cancelled | Failed _ -> false
;;

let history_row id =
  ( id
  , { Message.author = (if id mod 2 = 0 then "Assistant" else "You")
    ; body =
        Plain
          (sprintf
             "Saved message %d\n\
              Conversation history stays available while new responses stream."
             id)
    ; detail = "Saved history"
    } )
;;

let create app ~scope:parent ~id ~title ~sleep =
  let open Or_error.Let_syntax in
  let%bind scope = Scope.child parent ~name:(sprintf "conversation-%d" id) in
  let items =
    Collection.of_alist (module Int) (List.init 40 ~f:(fun n -> history_row (161 + n)))
    |> Or_error.ok_exn
  in
  let pager_result =
    Pager.create ~scope items ~before:(More (Some "161")) ~after:End ~load:(fun request ->
      sleep 0.15;
      match Pager.Request.direction request with
      | After -> Ok { Pager.Page.rows = []; next = End }
      | Before ->
        let cursor = Pager.Request.cursor request |> Option.value_exn |> Int.of_string in
        let first = Int.max 1 (cursor - 40) in
        Ok
          { Pager.Page.rows =
              List.init (cursor - first) ~f:(fun n -> history_row (first + n))
          ; next = (if first = 1 then End else More (Some (Int.to_string first)))
          })
  in
  let%map pager =
    Result.map_error pager_result ~f:(fun error ->
      Scope.cancel scope;
      error)
  in
  { app
  ; scope
  ; id
  ; title
  ; sleep
  ; pager
  ; phase = B.Expert.Var.create Phase.Idle
  ; active = None
  ; token = 0
  ; next_row = 201
  ; responses = 0
  ; last = None
  ; attachments = 0
  ; initialized = false
  }
;;

let current t token =
  Scope.is_active t.scope
  && Option.exists t.active ~f:(fun active ->
    active.token = token && Scope.is_active active.scope)
;;

let update_response t response detail =
  Pager.set
    t.pager
    ~key:response.Response.row
    ~data:
      { Message.author = "Assistant"; body = Rich (response.document, Markdown); detail }
  |> Or_error.ok_exn
;;

let cancel t =
  match t.active with
  | None -> ()
  | Some active ->
    t.active <- None;
    Scope.cancel active.scope;
    (match active.response with
     | None -> set_phase t Idle
     | Some response ->
       Document.cancel response.document |> Or_error.ok_exn;
       update_response t response "Cancelled — partial response retained";
       set_phase t Cancelled)
;;

let finish t token response result =
  if current t token
  then (
    let active = Option.value_exn t.active in
    t.active <- None;
    Scope.cancel active.scope;
    match result with
    | Ok () ->
      Document.finish response.Response.document |> Or_error.ok_exn;
      update_response t response "Complete";
      set_phase t Complete
    | Error error ->
      Document.cancel response.document |> Or_error.ok_exn;
      let error = Error.to_string_hum error in
      update_response t response error;
      set_phase t (Failed error))
;;

let start_stream t token response config =
  let active = Option.value_exn t.active in
  set_phase t Streaming;
  update_response t response "Streaming";
  match
    Scope.start
      active.scope
      ~f:(fun () ->
        let rec loop = function
          | [] -> Or_error.error_string "fake backend omitted terminal state"
          | Backend.Step.Finish :: _ -> Ok ()
          | Fail error :: _ -> Or_error.error_string error
          | Chunk bytes :: rest ->
            t.sleep (Backend.Config.delay_seconds config);
            if current t token
            then (
              Document.push_bytes response.document bytes |> Or_error.ok_exn;
              loop rest)
            else Or_error.error_string "obsolete response"
        in
        loop (Backend.plan config ~prompt:response.prompt))
      ~on_result:(fun result ->
        E.of_thunk (fun () -> finish t token response (Or_error.join result)))
  with
  | Ok (_ : Scope.Task.t) -> ()
  | Error error -> finish t token response (Error error)
;;

let new_token t =
  t.token <- t.token + 1;
  t.token
;;

let accept_response t ~token ~acceptance ~document ~prompt ~config =
  let open Or_error.Let_syntax in
  let result =
    let%bind producer = Scope.child t.scope ~name:"response" in
    let row = t.next_row + 1 in
    let response = { Response.document; row; prompt } in
    let appended =
      Pager.append
        t.pager
        [ row - 1, { Message.author = "You"; body = Plain prompt; detail = "Sent" }
        ; ( row
          , { Message.author = "Assistant"
            ; body = Rich (document, Markdown)
            ; detail = "Streaming"
            } )
        ]
    in
    let%map () =
      Result.map_error appended ~f:(fun error ->
        Scope.cancel producer;
        error)
    in
    t.active <- Some { Active.token; scope = producer; response = Some response };
    Scope.cancel acceptance;
    t.last <- Some response;
    t.responses <- t.responses + 1;
    t.next_row <- row + 1;
    start_stream t token response config
  in
  Result.iter_error result ~f:(fun error ->
    Document.release document;
    cancel t;
    set_phase t (Failed (Error.to_string_hum error)));
  result
;;

let submit t ~window_scope ~config ~prompt ~on_accept =
  if is_busy t
  then Or_error.error_string "A response is already running in this conversation."
  else if t.responses >= 64
  then Or_error.error_string "This demo conversation has reached its 64-response limit."
  else if
    String.is_empty (String.strip prompt)
    || String.length prompt > 16384
    || (not (Stdlib.String.is_valid_utf_8 prompt))
    || String.contains prompt '\000'
  then Or_error.error_string "Enter a UTF-8 prompt of 1–16384 bytes without NUL."
  else
    let open Or_error.Let_syntax in
    let%bind acceptance = Scope.child window_scope ~name:"send-acceptance" in
    let token = new_token t in
    t.active <- Some { Active.token; scope = acceptance; response = None };
    set_phase t Accepting;
    let cleanup =
      Scope.Expert.on_cancel acceptance (fun () ->
        match t.active with
        | Some active when active.token = token && Option.is_none active.response ->
          t.active <- None;
          set_phase t Idle
        | Some _ | None -> ())
    in
    let%bind _unregister =
      Result.map_error cleanup ~f:(fun error ->
        cancel t;
        error)
    in
    match
      Scope.start
        acceptance
        ~f:(fun () -> t.sleep (Backend.Config.accept_delay_seconds config))
        ~on_result:(fun result ->
          let open E.Let_syntax in
          match result with
          | Error error ->
            E.of_thunk (fun () ->
              if current t token
              then (
                cancel t;
                set_phase t (Failed (Error.to_string_hum error))))
          | Ok () ->
            let%bind created =
              Document.create t.app ~scope:t.scope (Source.empty_stream ())
            in
            if not (current t token)
            then E.of_thunk (fun () -> Result.iter created ~f:Document.release)
            else (
              match created with
              | Error error ->
                E.of_thunk (fun () ->
                  cancel t;
                  set_phase
                    t
                    (Failed (Sexp.to_string_hum (Document.Error.sexp_of_t error))))
              | Ok document ->
                let%bind accepted =
                  E.of_thunk (fun () ->
                    accept_response t ~token ~acceptance ~document ~prompt ~config)
                in
                (match accepted with
                 | Ok () -> on_accept
                 | Error _ -> E.Ignore)))
    with
    | Ok (_ : Scope.Task.t) -> Ok ()
    | Error error ->
      cancel t;
      Error error
;;

let retry t ~config =
  if is_busy t
  then Or_error.error_string "A response is already running."
  else (
    match t.last with
    | None -> Or_error.error_string "There is no response to retry."
    | Some response ->
      let open Or_error.Let_syntax in
      let%bind scope = Scope.child t.scope ~name:"response-retry" in
      let%bind () =
        Result.map_error (Document.reset response.document "") ~f:(fun error ->
          Scope.cancel scope;
          error)
      in
      let token = new_token t in
      t.active <- Some { Active.token; scope; response = Some response };
      start_stream t token response config;
      Ok ())
;;

let initialize t =
  let open E.Let_syntax in
  let%bind start =
    E.of_thunk (fun () ->
      if t.initialized
      then false
      else (
        t.initialized <- true;
        true))
  in
  if not start
  then E.Ignore
  else (
    let rec loop = function
      | [] -> E.Ignore
      | (row, text, mode, label) :: rest ->
        let source = Source.of_string text |> Or_error.ok_exn in
        let%bind result = Document.create t.app ~scope:t.scope source in
        let%bind () =
          E.of_thunk (fun () ->
            match result with
            | Error error ->
              if Scope.is_active t.scope
              then
                Pager.set
                  t.pager
                  ~key:row
                  ~data:
                    { Message.author = "Artifact"
                    ; body = Plain (Sexp.to_string_hum (Document.Error.sexp_of_t error))
                    ; detail = "Preview unavailable"
                    }
                |> Or_error.ok_exn
            | Ok document ->
              if Scope.is_active t.scope
              then
                Pager.set
                  t.pager
                  ~key:row
                  ~data:
                    { Message.author = "Tool · workspace.inspect"
                    ; body = Rich (document, mode)
                    ; detail = label
                    }
                |> Or_error.ok_exn
              else Document.release document)
        in
        loop rest
    in
    loop
      [ 198, Backend.markdown_fixture, Gpuio.Document.Mode.Markdown, "Workspace notes"
      ; 199, Backend.code_fixture, Code Gpuio.Document.Language.ocaml, "greeting.ml"
      ; 200, Backend.diff_fixture, Diff, "Proposed patch"
      ])
;;

let attach t ~name ~text =
  let open E.Let_syntax in
  let%bind admitted =
    E.of_thunk (fun () ->
      if not (Scope.is_active t.scope)
      then Or_error.error_string "Conversation closed."
      else if
        t.attachments >= 8 || String.length text > 65536 || String.length name > 4096
      then
        Or_error.error_string
          "Attachment limit: eight UTF-8 text files, up to 64 KiB each."
      else if
        (not (Stdlib.String.is_valid_utf_8 text))
        || String.contains text '\000'
        || String.is_empty name
        || (not (Stdlib.String.is_valid_utf_8 name))
        || String.contains name '\000'
      then Or_error.error_string "This demo previews UTF-8 text attachments."
      else (
        t.attachments <- t.attachments + 1;
        match Source.of_string text with
        | Ok source -> Ok source
        | Error error ->
          t.attachments <- t.attachments - 1;
          Error error))
  in
  match admitted with
  | Error error -> E.return (Error error)
  | Ok source ->
    let%bind result = Document.create t.app ~scope:t.scope source in
    E.of_thunk (fun () ->
      match result with
      | Error error ->
        t.attachments <- t.attachments - 1;
        Or_error.error_s (Document.Error.sexp_of_t error)
      | Ok document ->
        let row = t.next_row in
        t.next_row <- row + 1;
        Pager.append
          t.pager
          [ ( row
            , { Message.author = "Attachment"
              ; body = Rich (document, Code Gpuio.Document.Language.plain_text)
              ; detail = sprintf "%s · %d bytes" name (String.length text)
              } )
          ]
        |> Result.map_error ~f:(fun error ->
          Document.release document;
          t.attachments <- t.attachments - 1;
          error))
;;
