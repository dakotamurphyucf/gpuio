open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Scope = Gpuio_eio.Scope
module Selection = Gpuio_gallery_model.Theme_selection
module Profile = Gpuio_gallery_model.Theme_profile

let ok = Or_error.ok_exn

let describe_error error =
  let message = Error.to_string_hum error in
  if not (Stdlib.String.is_valid_utf_8 message)
  then "Could not read this theme file."
  else (
    let rec prefix length =
      let text = String.prefix message length in
      if Stdlib.String.is_valid_utf_8 text then text else prefix (length - 1)
    in
    prefix (Int.min 512 (String.length message)))
;;

let component ~load ~selection window palette graph =
  let draft =
    Gpuio_eio.Text_input.create
      window
      ~initial_text:"This draft stays while the palette changes."
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Theme preview draft" ()
            |> ok))
      graph
  in
  let busy = B.Expert.Var.create false in
  let path = B.Expert.Var.create None in
  let status =
    B.Expert.Var.create "Choose a profile, then reload it after saving changes."
  in
  let scope =
    Preview_scope.acquire
      window
      ~name:"theme-file"
      ~create:(fun scope ->
        E.of_thunk (fun () ->
          Scope.on_cancel scope (fun () ->
            if B.Expert.Var.get busy
            then B.Expert.Var.set status "Loading cancelled. Your theme was kept.";
            B.Expert.Var.set busy false)
          |> Or_error.map ~f:(fun _ -> scope)))
      graph
  in
  let request scope ~reload =
    let open E.Let_syntax in
    let%bind token =
      E.of_thunk (fun () ->
        if (not (Scope.is_active scope)) || B.Expert.Var.get busy
        then None
        else (
          B.Expert.Var.set busy true;
          Some (Selection.token (B.Expert.Var.get selection))))
    in
    match token with
    | None -> E.Ignore
    | Some token ->
      let%bind chosen =
        if reload
        then E.of_thunk (fun () -> Ok (Option.map (B.Expert.Var.get path) ~f:List.return))
        else
          Gpuio_eio.File_dialog.open_
            window
            ~config:
              (File_dialog.Open.create
                 ~title:"Choose a theme profile"
                 ~accept_label:"Load theme"
                 ()
               |> ok)
      in
      E.of_thunk (fun () ->
        if Scope.is_active scope
        then (
          let finish message =
            B.Expert.Var.set busy false;
            B.Expert.Var.set status message
          in
          match chosen with
          | Error _ -> finish "The file picker could not open. Your theme was kept."
          | Ok None -> finish "Selection cancelled. Your theme was kept."
          | Ok (Some [ _ ])
            when not
                   (Selection.Token.equal
                      token
                      (Selection.token (B.Expert.Var.get selection))) ->
            finish "A newer appearance choice was kept."
          | Ok (Some [ file ]) ->
            B.Expert.Var.set path (Some file);
            B.Expert.Var.set status "Loading theme…";
            (match
               Scope.start
                 scope
                 ~f:(fun () -> load file)
                 ~on_result:(fun result ->
                   E.of_thunk (fun () ->
                     if Scope.is_active scope
                     then (
                       let current = B.Expert.Var.get selection in
                       if not (Selection.Token.equal token (Selection.token current))
                       then finish "A newer appearance choice was kept."
                       else (
                         let result = Or_error.join result in
                         B.Expert.Var.set
                           selection
                           (Selection.complete current ~token result);
                         finish
                           (match result with
                            | Ok profile -> "Loaded " ^ Profile.name profile
                            | Error error -> "Theme kept. " ^ describe_error error)))))
             with
             | Ok _ -> ()
             | Error error -> finish (describe_error error))
          | Ok (Some _) -> finish "Choose exactly one profile. Your theme was kept."))
  in
  let open B.Let_syntax in
  let%arr p = palette
  and scope = scope
  and busy = B.Expert.Var.value busy
  and path = B.Expert.Var.value path
  and status = B.Expert.Var.value status
  and selected = B.Expert.Var.value selection
  and draft = draft in
  let active, choose, reload =
    match scope with
    | Preview_scope.Ready scope ->
      true, request scope ~reload:false, request scope ~reload:true
    | Loading | Failed _ -> false, E.Ignore, E.Ignore
  in
  let status =
    match scope with
    | Preview_scope.Failed error -> "Theme loading unavailable. " ^ describe_error error
    | Loading -> "Preparing theme loading…"
    | Ready _ -> status
  in
  Palette.card
    p
    ~title:"A theme from your workspace"
    [ Palette.text
        p
        ~muted:true
        "Load a .sexp palette for this window. Reload is explicit; invalid files keep \
         the last working theme."
    ; Palette.text
        p
        (match Selection.profile selected with
         | None -> "Using the built-in appearance"
         | Some profile -> "Current profile: " ^ Profile.name profile)
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 8.); Wrap Wrap ])
        [ Palette.button p ~disabled:(busy || not active) "Choose theme" choose
        ; Palette.button
            p
            ~disabled:(busy || (not active) || Option.is_none path)
            "Reload file"
            reload
        ]
    ; Gpuio_eio.Text_input.view
        draft
        ~style:
          (Style.create_exn
             [ Width (Length.percent_exn 100.)
             ; Height (Length.px_exn 40.)
             ; Padding (Length.px_exn 8.)
             ; Border_width 1.
             ; Radius 8.
             ; Border_color (Palette.border p)
             ; Foreground (Palette.foreground p)
             ])
    ; Palette.text p ~muted:true status
    ]
;;
