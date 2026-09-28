open Core
module Workspace = Signal_studio_model.Workspace
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Desktop = Gpuio_eio.Desktop
module Picker = Gpuio_eio.File_dialog
module E = Bonsai.Effect

module State = struct
  type t =
    { path : Gpuio.File_path.t option
    ; edited : bool
    ; busy : bool
    ; metadata : (unit, Gpuio.Window.Error.t) Result.t option
    }

  let initial = { path = None; edited = false; busy = false; metadata = None }
end

module Model = struct
  type t =
    { current : unit -> Workspace.t
    ; replace : Workspace.t -> unit
    ; stop_stream : unit -> unit
    ; report : string -> unit
    }
end

type t =
  { app : App.t
  ; model : Model.t
  ; window : unit -> App.Window.t option
  ; load : Gpuio.File_path.t -> Workspace.t Or_error.t
  ; save : Gpuio.File_path.t -> Workspace.t -> unit Or_error.t
  ; on_state : State.t -> unit
  ; mutable state : State.t
  ; mutable saved : string
  ; mutable epoch : int
  ; mutable metadata_revision : int
  }

let create app ~model ~window ~load ~save ~on_state =
  { app
  ; model
  ; window
  ; load
  ; save
  ; on_state
  ; state = State.initial
  ; saved = Workspace.encode (model.Model.current ())
  ; epoch = 0
  ; metadata_revision = 0
  }
;;

let state t = t.state

let notify t =
  t.state
  <- { t.state with
       edited = not (String.equal t.saved (Workspace.encode (t.model.current ())))
     };
  t.on_state t.state
;;

let refresh t =
  E.bind
    (E.of_thunk (fun () ->
       notify t;
       t.window ()))
    ~f:(function
      | None -> E.Ignore
      | Some window when App.Window.is_closed window -> E.Ignore
      | Some window ->
        let document =
          Desktop.Document.create ?path:t.state.path ~edited:t.state.edited ()
        in
        t.state <- { t.state with metadata = None };
        notify t;
        t.metadata_revision <- t.metadata_revision + 1;
        let revision = t.metadata_revision in
        E.map (Desktop.set_document window document) ~f:(fun result ->
          if revision = t.metadata_revision && not (App.Window.is_closed window)
          then (
            t.state
            <- { t.state with metadata = Some (Result.map result ~f:(fun _ -> ())) };
            notify t)))
;;

let reset t =
  E.bind
    (E.of_thunk (fun () ->
       t.epoch <- t.epoch + 1;
       t.saved <- Workspace.encode (t.model.current ());
       t.state <- { t.state with path = None }))
    ~f:(fun () -> refresh t)
;;

let begin_operation t =
  if t.state.busy
  then (
    t.model.report "A document operation is already in progress.";
    false)
  else (
    t.state <- { t.state with busy = true };
    notify t;
    true)
;;

let finish t =
  t.state <- { t.state with busy = false };
  notify t
;;

let start_io t ~operation ~f ~accept =
  let epoch = t.epoch in
  match
    Scope.start (App.scope t.app) ~f ~on_result:(fun result ->
      E.bind
        (E.of_thunk (fun () ->
           finish t;
           if epoch = t.epoch
           then (
             match Or_error.join result with
             | Ok result -> accept result
             | Error error ->
               Eio.traceln
                 "SIGNAL_STUDIO: document %s failed: %s"
                 operation
                 (Error.to_string_hum error);
               t.model.report ("Could not " ^ operation ^ " the workspace."))))
        ~f:(fun () -> refresh t))
  with
  | Ok (_ : Scope.Task.t) -> ()
  | Error error ->
    finish t;
    t.model.report ("Document task unavailable: " ^ Error.to_string_hum error)
;;

let save_admitted t path =
  let workspace = t.model.current () in
  let contents = Workspace.encode workspace in
  start_io
    t
    ~operation:"save"
    ~f:(fun () -> t.save path workspace)
    ~accept:(fun () ->
      t.saved <- contents;
      t.state <- { t.state with path = Some path };
      t.model.report "Workspace saved.")
;;

let load_admitted ?before t path =
  t.model.stop_stream ();
  let before = Option.value before ~default:(Workspace.encode (t.model.current ())) in
  start_io
    t
    ~operation:"load"
    ~f:(fun () -> t.load path)
    ~accept:(fun workspace ->
      if String.equal before (Workspace.encode (t.model.current ()))
      then (
        t.model.replace workspace;
        t.saved <- Workspace.encode workspace;
        t.state <- { t.state with path = Some path };
        t.model.report "Workspace loaded.")
      else t.model.report "Load abandoned: the workspace changed while reading.")
;;

let save_path t path =
  E.of_thunk (fun () -> if begin_operation t then save_admitted t path)
;;

let load_path t path =
  E.of_thunk (fun () -> if begin_operation t then load_admitted t path)
;;

let with_picker t picker accept =
  E.bind
    (E.of_thunk (fun () ->
       if begin_operation t
       then (
         match t.window () with
         | Some window when not (App.Window.is_closed window) -> Some (window, t.epoch)
         | None | Some _ ->
           finish t;
           t.model.report "Open a workspace window first.";
           None)
       else None))
    ~f:(function
      | None -> E.Ignore
      | Some (window, epoch) ->
        E.map (picker window) ~f:(function
          | Ok (Some value) ->
            if epoch = t.epoch
            then accept value
            else (
              finish t;
              t.model.report "Document operation superseded by reset.")
          | Ok None ->
            finish t;
            t.model.report "Document operation cancelled."
          | Error error ->
            finish t;
            t.model.report
              ("File picker: " ^ Sexp.to_string ([%sexp_of: Picker.Error.t] error))))
;;

let open_ t ~directory =
  E.bind
    (E.of_thunk (fun () ->
       notify t;
       if t.state.edited
       then (
         t.model.report "Save or reset changes before opening another workspace.";
         None)
       else (
         t.model.stop_stream ();
         Some (Workspace.encode (t.model.current ())))))
    ~f:(function
      | None -> E.Ignore
      | Some before ->
        with_picker
          t
          (fun window ->
             E.map
               (Picker.open_
                  window
                  ~config:
                    (Picker.Open.create ~title:"Open workspace" ~directory ()
                     |> Or_error.ok_exn))
               ~f:(Result.map ~f:(Option.bind ~f:List.hd)))
          (load_admitted ~before t))
;;

let save_as t ~directory =
  with_picker
    t
    (fun window ->
       Picker.save
         window
         ~config:
           (Picker.Save.create
              ~directory
              ~suggested_name:"workspace.signal"
              ~title:"Save workspace"
              ()
            |> Or_error.ok_exn))
    (save_admitted t)
;;

let reveal t =
  E.bind
    (E.of_thunk (fun () -> t.state.path))
    ~f:(function
      | None ->
        E.of_thunk (fun () -> t.model.report "Save a workspace before revealing it.")
      | Some path ->
        E.map (Desktop.reveal_file t.app path) ~f:(function
          | Ok () -> t.model.report "Reveal requested."
          | Error error ->
            t.model.report
              ("Reveal: " ^ Sexp.to_string ([%sexp_of: Desktop.Error.t] error))))
;;
